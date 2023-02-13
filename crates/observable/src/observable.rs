use std::{cell::Ref, cell::RefCell, rc::Rc};

#[derive(Default)]
struct ListenerSet<T> {
    nextid: usize,
    items: Vec<ListenerItem<T>>,
}

struct ListenerItem<T> {
    /// Monotonic id for use in the binary search
    id: usize,
    listener: Listener<T>,
}

pub enum Listener<T> {
    Once(Box<dyn Fn(&T)>),
    Durable(Rc<RefCell<Box<dyn Fn(&T)>>>),
}

pub struct ListenerHandle(usize);

pub struct Observable<T> {
    value: Rc<RefCell<T>>,
    listener_set: Rc<RefCell<ListenerSet<T>>>,
}

// Implemented manually because `T` does not need to be Clone
impl<T> Clone for Observable<T> {
    fn clone(&self) -> Self {
        Observable {
            value: self.value.clone(),
            listener_set: self.listener_set.clone(),
        }
    }
}

impl<T> Observable<T> {
    pub fn new(value: T) -> Self {
        Self {
            value: Rc::new(RefCell::new(value)),
            listener_set: Rc::new(RefCell::new(ListenerSet {
                nextid: 0,
                items: Vec::new(),
            })),
        }
    }
    fn notify(&self) {
        let mut working_set: Vec<Listener<T>>;
        {
            let mut listenerset = self.listener_set.borrow_mut();
            // It's possible to add listeners while we are firing a listener
            // so we need to make a copy of the listeners vec so we're not mutating it while calling listener functions

            working_set = Vec::with_capacity(listenerset.items.len());

            // Take all Listener::Once entries, and clone the others
            let mut i = 0;
            while i != listenerset.items.len() {
                match listenerset.items[i].listener {
                    Listener::Once(_) => {
                        // Just take it
                        working_set.push(listenerset.items.remove(i).listener);
                    }
                    Listener::Durable(ref f) => {
                        working_set.push(Listener::Durable(f.clone()));
                        i += 1;
                    }
                }
            }
        }

        let r = self.get();

        // Now that the borrow on the listeners vec is over, we can safely call them
        // We can also be confident that we won't call any listeners which were attached during our dispatch
        for listener in working_set {
            match listener {
                Listener::Once(f) => f(&r),
                Listener::Durable(f) => {
                    (f.borrow_mut())(&r);
                }
            }
        }
    }

    fn _subscribe(&self, listener: Listener<T>) -> ListenerHandle {
        let mut listener_set = self.listener_set.borrow_mut();

        let id = listener_set.nextid;
        listener_set.nextid += 1;
        listener_set.items.push(ListenerItem { id, listener });
        ListenerHandle(id)
    }

    // impl<T> Set<T> for Observable<T> {
    pub fn set(&self, value: T) {
        {
            *(self.value.borrow_mut()) = value;
        };

        self.notify();
    }
    // }
    // impl<T> Observe<T> for Observable<T> {
    pub fn get(&self) -> Ref<T> {
        self.value.borrow()
    }
    pub fn subscribe(&self, cb: Box<(dyn Fn(&T))>) -> ListenerHandle {
        let listener = Listener::Durable(Rc::new(RefCell::new(cb.into())));
        self._subscribe(listener)
    }
    pub fn once(&self, cb: Box<(dyn Fn(&T))>) -> ListenerHandle {
        let listener = Listener::Once(cb.into());
        self._subscribe(listener)
    }

    pub fn unsubscribe(&self, handle: ListenerHandle) -> bool {
        let mut listener_set = self.listener_set.borrow_mut();

        // Find the current listener offset
        match listener_set
            .items
            .binary_search_by(|probe| probe.id.cmp(&handle.0))
        {
            Ok(offset) => {
                listener_set.items.remove(offset);
                true
            }
            Err(_) => false,
        }
    }
}
impl<T> Default for Observable<T>
where
    T: Default,
{
    fn default() -> Self {
        Observable::new(T::default())
    }
}

impl<T> Observable<Vec<T>> {
    pub fn push(&mut self, item: T) {
        self.value.borrow_mut().push(item);
        self.notify();
    }
}
