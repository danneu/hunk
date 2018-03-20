// Gateware is the first middleware to touch the request and the last middleware
// to touch the response.

use futures::Future;
use hyper::{Request, Response, server::Service};

#[derive(Debug)]
pub struct Gate<T> {
    next: T,
}

impl<T> Gate<T> {
    pub fn new(next: T) -> Self where T: Service + 'static {
        Gate { next }
    }
}

impl<T> Service for Gate<T> where T: Service<Request = Request, Response = Response> + 'static {
    type Request = T::Request;
    type Response = T::Response;
    type Error = T::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        Box::new(self.next.call(req).map(move |mut res: Response| {
            res.headers_mut().set_raw("Server", "Hunk");
            res
        }))
    }
}
