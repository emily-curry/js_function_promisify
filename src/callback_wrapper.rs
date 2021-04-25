use crate::callback_kind::CallbackKind;
use crate::closure_kind::ClosureKind;
use js_sys::Function;
use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;
use std::task::Poll;
use std::task::Waker;
use wasm_bindgen::closure::WasmClosureFnOnce;

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsValue;

#[derive(Debug)]
pub struct Callback<F: ?Sized> {
  closure: Closure<F>,
}

impl<F: ?Sized> Callback<F> {
  pub fn new(closure: Closure<F>) -> Self
  where
    F: 'static,
  {
    Self { closure }
  }

  pub fn to_function(&self) -> Function {
    let js_func: JsValue = self.closure.as_ref().into();
    let func: Function = js_func.into();
    func
  }
}

trait CallbackMarker {}

impl CallbackMarker for Callback<dyn FnMut(JsValue)> {}

fn finish(state: &RefCell<CallbackWrapperInner>, val: Result<JsValue, JsValue>) {
  let task = {
    let mut state = state.borrow_mut();
    debug_assert!(state.result.is_none());
    for cb in state.cb.to_owned().into_iter() {
      drop(cb);
    }
    state.result = Some(val);
    state.task.take()
  };

  if let Some(task) = task {
    task.wake()
  }
}

// #[derive(Debug)]
pub struct CallbackWrapper {
  inner: Rc<RefCell<CallbackWrapperInner>>,
}

impl CallbackWrapper {
  /// Creates a new `CallbackWrapper` ... TODO:
  pub fn new() -> Self {
    Self {
      inner: CallbackWrapperInner::new(),
    }
  }

  pub fn get_args1<F>(&self, mut cb: F) -> Rc<Callback<dyn FnMut(JsValue)>>
  where
    F: 'static + FnMut(JsValue) -> Result<JsValue, JsValue>,
  {
    let state = Rc::clone(&self.inner);
    let closure = Closure::once(move |a1| finish(&state, cb(a1)));
    let callback = Callback::new(closure);
    self.register_callback(callback)
  }

  fn register_callback<F>(&self, cb: F) -> Rc<F>
  where
    F: 'static + CallbackMarker,
  {
    let ptr = Rc::new(cb);
    let ret = Rc::clone(&ptr);
    let mut state = self.inner.borrow_mut();
    state.cb.push(ptr);
    ret
  }

  // pub fn get(&self) -> Rc<ClosureKind> {
  //   self.get_closure(CallbackKind::Arg1(Box::new(|x| Ok(x))))
  // }

  // pub fn get_closure(&self, callback: CallbackKind) -> Rc<ClosureKind> {
  //   let state = self.inner.clone();
  //   let closure_kind = match callback {
  //     CallbackKind::Arg0(mut f) => ClosureKind::Arg0(Closure::once(move || finish(&state, f()))),
  //     CallbackKind::Arg1(mut f) => {
  //       ClosureKind::Arg1(Closure::once(move |a1| finish(&state, f(a1))))
  //     }
  //     CallbackKind::Arg2(mut f) => {
  //       ClosureKind::Arg2(Closure::once(move |a1, a2| finish(&state, f(a1, a2))))
  //     }
  //     CallbackKind::Arg3(mut f) => ClosureKind::Arg3(Closure::once(move |a1, a2, a3| {
  //       finish(&state, f(a1, a2, a3))
  //     })),
  //     CallbackKind::Arg4(mut f) => ClosureKind::Arg4(Closure::once(move |a1, a2, a3, a4| {
  //       finish(&state, f(a1, a2, a3, a4))
  //     })),
  //     CallbackKind::Arg5(mut f) => ClosureKind::Arg5(Closure::once(move |a1, a2, a3, a4, a5| {
  //       finish(&state, f(a1, a2, a3, a4, a5))
  //     })),
  //   };
  //   let ptr = Rc::new(closure_kind);
  //   let ret = Rc::clone(&ptr);
  //   let mut state = self.inner.borrow_mut();
  //   state.cb.push(ptr);
  //   ret
  // }

  // pub fn get_resolve(&self) -> Rc<ClosureKind> {
  //   self.get_closure(CallbackKind::Arg1(Box::new(|x| Ok(x))))
  // }

  // pub fn get_reject(&self) -> Rc<ClosureKind> {
  //   self.get_closure(CallbackKind::Arg1(Box::new(|x| Err(x))))
  // }

  // pub fn get_node(&self) -> Rc<ClosureKind> {
  //   self.get_closure(CallbackKind::Arg2(Box::new(|err, data| {
  //     if err == JsValue::UNDEFINED || err == JsValue::NULL {
  //       return Ok(data);
  //     }
  //     Err(err)
  //   })))
  // }
}

impl Future for CallbackWrapper {
  type Output = Result<JsValue, JsValue>;

  fn poll(
    self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> std::task::Poll<Self::Output> {
    let mut inner = self.inner.borrow_mut();
    if let Some(val) = inner.result.take() {
      return Poll::Ready(val);
    }
    inner.task = Some(cx.waker().clone());
    Poll::Pending
  }
}

// #[derive(Debug)]
struct CallbackWrapperInner {
  cb: Vec<Rc<dyn CallbackMarker>>,
  result: Option<Result<JsValue, JsValue>>,
  task: Option<Waker>,
}

impl CallbackWrapperInner {
  fn new() -> Rc<RefCell<CallbackWrapperInner>> {
    Rc::new(RefCell::new(CallbackWrapperInner {
      cb: Vec::new(),
      task: None,
      result: None,
    }))
  }
}
