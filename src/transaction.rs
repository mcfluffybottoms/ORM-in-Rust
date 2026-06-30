use crate::Error::NotFound;
use crate::Result;
use crate::{
    data::ObjectId,
    error::*,
    object::{Object, Store},
    storage::StorageTransaction,
};
use std::{
    any::{Any, TypeId},
    cell::{Ref, RefCell, RefMut},
    collections::{HashMap, HashSet},
    marker::PhantomData,
    rc::Rc,
};

////////////////////////////////////////////////////////////////////////////////

pub struct Transaction<'a> {
    inner: Box<dyn StorageTransaction + 'a>,
    objects: RefCell<HashMap<(TypeId, ObjectId), Rc<dyn Any>>>,
    storeobjects: RefCell<HashMap<(TypeId, ObjectId), Rc<RefCell<dyn Store>>>>,
    dirty_objects: RefCell<HashSet<(TypeId, ObjectId)>>,
    removed_objects: RefCell<Vec<(TypeId, ObjectId)>>,
}

pub fn get_type_name<T: Object>() -> &'static str {
    T::schema().type_name
}

impl<'a> Transaction<'a> {
    pub(crate) fn new(inner: Box<dyn StorageTransaction + 'a>) -> Self {
        Self {
            inner,
            objects: RefCell::new(HashMap::new()),
            storeobjects: RefCell::new(HashMap::new()),
            dirty_objects: RefCell::new(HashSet::new()),
            removed_objects: RefCell::new(Vec::new()),
        }
    }

    pub fn create<T: Object>(&mut self, obj: T) -> Result<Tx<'_, T>> {
        let schema = T::schema();
        if !self.inner.table_exists(schema.name)? {
            self.inner.create_table(schema)?;
        }

        let columns = self.inner.load_columns(schema)?;
        for field in schema.fields {
            if !columns.contains(&field.column_name.to_string()) {
                return Err(Error::MissingColumn(Box::new(MissingColumnError {
                    type_name: get_type_name::<T>(),
                    table_name: schema.name,
                    attr_name: field.name,
                    column_name: field.column_name,
                })));
            }
        }

        let db_id = self.inner.insert_row(schema, &obj.as_row())?;

        let obj_rc = self.insert_into_temp_collection(obj, db_id);

        self.dirty_objects
            .borrow_mut()
            .insert((TypeId::of::<T>(), db_id));

        Ok(Tx {
            id: db_id,
            obj: obj_rc,
            transaction: self,
            lifetime: PhantomData,
        })
    }

    pub fn get<T: Object>(&mut self, id: ObjectId) -> Result<Tx<'_, T>> {
        let schema = T::schema();
        if !self.inner.table_exists(schema.name)? {
            self.inner.create_table(schema)?;
        }

        // the value was removed in a previous transaction
        if self
            .removed_objects
            .borrow()
            .contains(&(TypeId::of::<T>(), id))
        {
            return Err(NotFound(Box::new(crate::error::NotFoundError {
                object_id: id,
                type_name: get_type_name::<T>(),
            })));
        }

        // get from cache
        if let Some(any_rc) = self.objects.borrow().get(&(TypeId::of::<T>(), id)) {
            match any_rc.clone().downcast::<RefCell<T>>() {
                Ok(obj_rc) => {
                    return Ok(Tx {
                        id,
                        obj: obj_rc,
                        transaction: self,
                        lifetime: PhantomData,
                    });
                }
                Err(_) => {
                    return Err(Error::Storage(Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "type mismatch in cache",
                    ))));
                }
            }
        }

        // if transaction does not have some fields it cannot be added
        let columns = self.inner.load_columns(schema)?;
        for field in schema.fields {
            if !columns.contains(&field.column_name.to_string()) {
                return Err(Error::MissingColumn(Box::new(MissingColumnError {
                    type_name: get_type_name::<T>(),
                    table_name: schema.name,
                    attr_name: field.name,
                    column_name: field.column_name,
                })));
            }
        }

        // get data
        let row = self.inner.select_row(id, schema)?;
        let obj: T = T::as_object(&row);

        // insert into cache
        let obj_rc = self.insert_into_temp_collection(obj, id);

        Ok(Tx {
            id,
            obj: obj_rc,
            transaction: self,
            lifetime: PhantomData,
        })
    }

    pub fn commit(&mut self) -> Result<()> {
        for (type_id, id) in self.dirty_objects.borrow().iter() {
            if let Some(store_rc) = self.storeobjects.borrow().get(&(*type_id, *id)) {
                let store_ref = &(**store_rc).borrow();
                self.inner
                    .update_row(*id, store_ref.schema(), &store_ref.as_row())?;
            } else {
                println!("Could not update rows.")
            }
        }

        for (type_id, id) in self.removed_objects.borrow().iter() {
            if let Some(store_rc) = self.storeobjects.borrow().get(&(*type_id, *id)) {
                let store_ref = &(**store_rc).borrow();
                self.inner.delete_row(*id, store_ref.schema())?;
            } else {
                println!("Could not delete rows.")
            }
        }

        self.removed_objects.borrow_mut().clear();
        self.dirty_objects.borrow_mut().clear();

        self.inner.commit()?;
        Ok(())
    }

    pub fn rollback(&mut self) -> Result<()> {
        self.inner.rollback()?;
        Ok(())
    }

    fn mark_dirty<T: Any>(&mut self, id: ObjectId) {
        self.dirty_objects
            .borrow_mut()
            .insert((TypeId::of::<T>(), id));
    }

    fn mark_removed<T: Any>(&mut self, id: ObjectId) {
        self.removed_objects
            .borrow_mut()
            .push((TypeId::of::<T>(), id));
        self.dirty_objects
            .borrow_mut()
            .remove(&(TypeId::of::<T>(), id));
    }

    fn is_dirty<T: Any>(&mut self, id: ObjectId) -> bool {
        self.dirty_objects
            .borrow()
            .contains(&(TypeId::of::<T>(), id))
    }

    fn is_removed<T: Any>(&mut self, id: ObjectId) -> bool {
        self.removed_objects
            .borrow()
            .contains(&(TypeId::of::<T>(), id))
    }

    fn insert_into_temp_collection<T: Object>(&mut self, obj: T, id: ObjectId) -> Rc<RefCell<T>> {
        let obj_rc: Rc<RefCell<T>> = Rc::new(RefCell::new(obj));
        self.objects
            .borrow_mut()
            .insert((TypeId::of::<T>(), id), obj_rc.clone() as Rc<dyn Any>);
        self.storeobjects.borrow_mut().insert(
            (TypeId::of::<T>(), id),
            obj_rc.clone() as Rc<RefCell<dyn Store>>,
        );
        obj_rc
    }
}

//------------------------------------------------------------------------//

#[derive(Clone, Copy)]
pub enum ObjectState {
    Clean,
    Modified,
    Removed,
}

#[derive(Clone)]
pub struct Tx<'a, T> {
    id: ObjectId,
    obj: Rc<RefCell<T>>,
    transaction: &'a Transaction<'a>,
    lifetime: PhantomData<&'a T>,
}

impl<'a, T: Any> Tx<'a, T> {
    pub fn id(&mut self) -> ObjectId {
        self.id
    }

    pub fn state(&mut self) -> ObjectState {
        if self.transaction.is_removed::<T>(self.id) {
            ObjectState::Removed
        } else if self.transaction.is_dirty::<T>(self.id) {
            ObjectState::Modified
        } else {
            ObjectState::Clean
        }
    }

    pub fn borrow(&mut self) -> Ref<'_, T> {
        if self.transaction.is_removed::<T>(self.id) {
            panic!("Cannot borrow a removed object.");
        }
        (*self.obj).borrow()
    }

    pub fn borrow_mut(&mut self) -> RefMut<'_, T> {
        if self.transaction.is_removed::<T>(self.id) {
            panic!("Cannot borrow a removed object.");
        }
        self.transaction.mark_dirty::<T>(self.id);
        self.obj.borrow_mut()
    }

    pub fn delete(self) {
        if self.obj.try_borrow_mut().is_err() {
            panic!("cannot delete a borrowed object");
        }
        self.transaction.mark_removed::<T>(self.id);
    }
}
