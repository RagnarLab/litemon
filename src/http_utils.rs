//! Various utilities for the HTTP serve.
use std::convert::Infallible;

use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty};
use hyper::body::Bytes;
use hyper::Response;

/// Create a HTTP `404 Not Found` response.
pub(crate) fn not_found() -> Response<BoxBody<Bytes, Infallible>> {
    println!("not found");
    let mut res = Response::new(
        Empty::<Bytes>::new()
            .map_err(|never| match never {})
            .boxed(),
    );
    *res.status_mut() = hyper::StatusCode::NOT_FOUND;

    res
}

/// Create a HTTP `500 Internal Server Error` response.
pub(crate) fn internal_server_error() -> Response<BoxBody<Bytes, Infallible>> {
    println!("internal server error");
    let mut res = Response::new(
        Empty::<Bytes>::new()
            .map_err(|never| match never {})
            .boxed(),
    );
    *res.status_mut() = hyper::StatusCode::INTERNAL_SERVER_ERROR;

    res
}
