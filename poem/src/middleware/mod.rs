//! Commonly used middleware.

mod add_data;
mod catch_panic;
#[cfg(feature = "compression")]
mod compression;
#[cfg(feature = "cookie")]
mod cookie_jar_manager;
mod cors;
#[cfg(feature = "csrf")]
mod csrf;
mod force_https;
mod normalize_path;
#[cfg(feature = "opentelemetry")]
mod opentelemetry_metrics;
#[cfg(feature = "opentelemetry")]
mod opentelemetry_tracing;
mod propagate_header;
#[cfg(feature = "requestid")]
mod requestid;
mod sensitive_header;
mod set_header;
mod size_limit;
#[cfg(feature = "tokio-metrics")]
mod tokio_metrics_mw;
#[cfg(feature = "tower-compat")]
mod tower_compat;
mod tracing_mw;

use std::marker::PhantomData;

#[cfg(feature = "compression")]
pub use self::compression::{Compression, CompressionEndpoint};
#[cfg(feature = "cookie")]
pub use self::cookie_jar_manager::{CookieJarManager, CookieJarManagerEndpoint};
#[cfg(feature = "csrf")]
pub use self::csrf::{Csrf, CsrfEndpoint};
#[cfg(feature = "opentelemetry")]
pub use self::opentelemetry_metrics::{OpenTelemetryMetrics, OpenTelemetryMetricsEndpoint};
#[cfg(feature = "opentelemetry")]
pub use self::opentelemetry_tracing::{OpenTelemetryTracing, OpenTelemetryTracingEndpoint};
#[cfg(feature = "requestid")]
pub use self::requestid::{ReqId, RequestId, RequestIdEndpoint, ReuseId};
#[cfg(feature = "tokio-metrics")]
pub use self::tokio_metrics_mw::{TokioMetrics, TokioMetricsEndpoint};
#[cfg(feature = "tower-compat")]
pub use self::tower_compat::TowerLayerCompatExt;
pub use self::{
    add_data::{AddData, AddDataEndpoint},
    catch_panic::{CatchPanic, CatchPanicEndpoint, PanicHandler},
    cors::{Cors, CorsEndpoint},
    force_https::ForceHttps,
    normalize_path::{NormalizePath, NormalizePathEndpoint, TrailingSlash},
    propagate_header::{PropagateHeader, PropagateHeaderEndpoint},
    sensitive_header::{SensitiveHeader, SensitiveHeaderEndpoint},
    set_header::{SetHeader, SetHeaderEndpoint},
    size_limit::{SizeLimit, SizeLimitEndpoint},
    tracing_mw::{Tracing, TracingEndpoint},
};
use crate::endpoint::{EitherEndpoint, Endpoint};

/// Represents a middleware trait.
///
/// # Create your own middleware
///
/// ```
/// use poem::{
///     Endpoint, EndpointExt, Middleware, Request, Result, handler, test::TestClient, web::Data,
/// };
///
/// /// A middleware that extracts token from HTTP headers.
/// struct TokenMiddleware;
///
/// impl<E: Endpoint> Middleware<E> for TokenMiddleware {
///     type Output = TokenMiddlewareImpl<E>;
///
///     fn transform(&self, ep: E) -> Self::Output {
///         TokenMiddlewareImpl { ep }
///     }
/// }
///
/// /// The new endpoint type generated by the TokenMiddleware.
/// struct TokenMiddlewareImpl<E> {
///     ep: E,
/// }
///
/// const TOKEN_HEADER: &str = "X-Token";
///
/// /// Token data
/// #[derive(Clone)]
/// struct Token(String);
///
/// impl<E: Endpoint> Endpoint for TokenMiddlewareImpl<E> {
///     type Output = E::Output;
///
///     async fn call(&self, mut req: Request) -> Result<Self::Output> {
///         if let Some(value) = req
///             .headers()
///             .get(TOKEN_HEADER)
///             .and_then(|value| value.to_str().ok())
///         {
///             // Insert token data to extensions of request.
///             let token = value.to_string();
///             req.extensions_mut().insert(Token(token));
///         }
///
///         // call the next endpoint.
///         self.ep.call(req).await
///     }
/// }
///
/// #[handler]
/// async fn index(Data(token): Data<&Token>) -> String {
///     token.0.clone()
/// }
///
/// // Use the `TokenMiddleware` middleware to convert the `index` endpoint.
/// let ep = index.with(TokenMiddleware);
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let mut resp = TestClient::new(ep)
///     .get("/")
///     .header(TOKEN_HEADER, "abc")
///     .send()
///     .await;
/// resp.assert_status_is_ok();
/// resp.assert_text("abc").await;
/// # });
/// ```
///
/// # Create middleware with functions
///
/// ```rust
/// use std::sync::Arc;
///
/// use poem::{
///     Endpoint, EndpointExt, IntoResponse, Request, Result, handler, test::TestClient, web::Data,
/// };
/// const TOKEN_HEADER: &str = "X-Token";
///
/// #[handler]
/// async fn index(Data(token): Data<&Token>) -> String {
///     token.0.clone()
/// }
///
/// /// Token data
/// #[derive(Clone)]
/// struct Token(String);
///
/// async fn token_middleware<E: Endpoint>(next: E, mut req: Request) -> Result<E::Output> {
///     if let Some(value) = req
///         .headers()
///         .get(TOKEN_HEADER)
///         .and_then(|value| value.to_str().ok())
///     {
///         // Insert token data to extensions of request.
///         let token = value.to_string();
///         req.extensions_mut().insert(Token(token));
///     }
///
///     // call the next endpoint.
///     next.call(req).await
/// }
///
/// let ep = index.around(token_middleware);
/// let cli = TestClient::new(ep);
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let resp = cli.get("/").header(TOKEN_HEADER, "abc").send().await;
/// resp.assert_status_is_ok();
/// resp.assert_text("abc").await;
/// # });
/// ```
pub trait Middleware<E: Endpoint> {
    /// New endpoint type.
    ///
    /// If you don't know what type to use, then you can use
    /// [`BoxEndpoint`](crate::endpoint::BoxEndpoint), which will bring some
    /// performance loss, but it is insignificant.
    type Output: Endpoint;

    /// Transform the input [`Endpoint`] to another one.
    fn transform(&self, ep: E) -> Self::Output;

    /// Create a new middleware by combining two middlewares.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{
    ///     Endpoint, EndpointExt, Middleware, Request, Result, handler, middleware::SetHeader,
    /// };
    ///
    /// #[handler]
    /// fn index() -> &'static str {
    ///     "hello"
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), std::io::Error> {
    ///     let ep = index.with(
    ///         SetHeader::new()
    ///             .appending("myheader", "a")
    ///             .combine(SetHeader::new().appending("myheader", "b")),
    ///     );
    ///
    ///     let resp = ep.call(Request::default()).await.unwrap();
    ///     let values = resp
    ///         .headers()
    ///         .get_all("myheader")
    ///         .iter()
    ///         .flat_map(|value| value.to_str().ok())
    ///         .collect::<Vec<_>>();
    ///     assert_eq!(values, vec!["a", "b"]);
    ///     Ok(())
    /// }
    /// ```
    fn combine<T>(self, other: T) -> CombineMiddleware<Self, T, E>
    where
        T: Middleware<Self::Output> + Sized,
        Self: Sized,
    {
        CombineMiddleware {
            a: self,
            b: other,
            _mark: PhantomData,
        }
    }

    /// if `enable` is `true` then combine the middleware.
    ///
    /// # Example
    ///
    /// ```
    /// use poem::{
    ///     Endpoint, EndpointExt, Middleware, Request, Result, handler, middleware::SetHeader,
    /// };
    ///
    /// #[handler]
    /// fn index() -> &'static str {
    ///     "hello"
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), std::io::Error> {
    ///     let ep1 = index.with(
    ///         SetHeader::new()
    ///             .appending("myheader", "a")
    ///             .combine_if(false, SetHeader::new().appending("myheader", "b")),
    ///     );
    ///
    ///     let ep2 = index.with(
    ///         SetHeader::new()
    ///             .appending("myheader", "a")
    ///             .combine_if(true, SetHeader::new().appending("myheader", "b")),
    ///     );
    ///
    ///     let resp = ep1.call(Request::default()).await.unwrap();
    ///     let values = resp
    ///         .headers()
    ///         .get_all("myheader")
    ///         .iter()
    ///         .flat_map(|value| value.to_str().ok())
    ///         .collect::<Vec<_>>();
    ///     assert_eq!(values, vec!["a"]);
    ///
    ///     let resp = ep2.call(Request::default()).await.unwrap();
    ///     let values = resp
    ///         .headers()
    ///         .get_all("myheader")
    ///         .iter()
    ///         .flat_map(|value| value.to_str().ok())
    ///         .collect::<Vec<_>>();
    ///     assert_eq!(values, vec!["a", "b"]);
    ///     Ok(())
    /// }
    /// ```
    fn combine_if<T>(
        self,
        enable: bool,
        other: T,
    ) -> EitherMiddleware<Self, CombineMiddleware<Self, T, E>, E>
    where
        T: Middleware<Self::Output> + Sized,
        Self: Sized,
    {
        if !enable {
            EitherMiddleware::left(self)
        } else {
            EitherMiddleware::right(self.combine(other))
        }
    }
}

impl<E: Endpoint> Middleware<E> for () {
    type Output = E;

    #[inline]
    fn transform(&self, ep: E) -> Self::Output {
        ep
    }
}

impl<E: Endpoint, T: Middleware<E>> Middleware<E> for &T {
    type Output = T::Output;

    fn transform(&self, ep: E) -> Self::Output {
        T::transform(self, ep)
    }
}

/// A middleware that combines two middlewares.
pub struct CombineMiddleware<A, B, E> {
    a: A,
    b: B,
    _mark: PhantomData<E>,
}

impl<A, B, E> Middleware<E> for CombineMiddleware<A, B, E>
where
    A: Middleware<E>,
    B: Middleware<A::Output>,
    E: Endpoint,
{
    type Output = B::Output;

    #[inline]
    fn transform(&self, ep: E) -> Self::Output {
        self.b.transform(self.a.transform(ep))
    }
}

/// The enum `EitherMiddleware` with variants `Left` and `Right` is a general
/// purpose sum type with two cases.
pub enum EitherMiddleware<A, B, E> {
    /// A middleware of type `A`
    A(A, PhantomData<E>),
    /// B middleware of type `B`
    B(B, PhantomData<E>),
}

impl<A, B, E> EitherMiddleware<A, B, E> {
    /// Create a new `EitherMiddleware` with the left variant.
    #[inline]
    pub fn left(a: A) -> Self {
        EitherMiddleware::A(a, PhantomData)
    }

    /// Create a new `EitherMiddleware` with the right variant.
    #[inline]
    pub fn right(b: B) -> Self {
        EitherMiddleware::B(b, PhantomData)
    }
}

impl<A, B, E> Middleware<E> for EitherMiddleware<A, B, E>
where
    A: Middleware<E>,
    B: Middleware<E>,
    E: Endpoint,
{
    type Output = EitherEndpoint<A::Output, B::Output>;

    #[inline]
    fn transform(&self, ep: E) -> Self::Output {
        match self {
            EitherMiddleware::A(a, _) => EitherEndpoint::A(a.transform(ep)),
            EitherMiddleware::B(b, _) => EitherEndpoint::B(b.transform(ep)),
        }
    }
}

poem_derive::generate_implement_middlewares!();

/// A middleware implemented by a closure.
pub struct FnMiddleware<T>(T);

impl<T, E, E2> Middleware<E> for FnMiddleware<T>
where
    T: Fn(E) -> E2,
    E: Endpoint,
    E2: Endpoint,
{
    type Output = E2;

    fn transform(&self, ep: E) -> Self::Output {
        (self.0)(ep)
    }
}

/// Make middleware with a closure.
pub fn make<T>(f: T) -> FnMiddleware<T> {
    FnMiddleware(f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        EndpointExt, IntoResponse, Request, Response, Result, handler,
        http::{HeaderValue, header::HeaderName},
        test::TestClient,
        web::Data,
    };

    #[tokio::test]
    async fn test_make() {
        #[handler(internal)]
        fn index() -> &'static str {
            "abc"
        }

        struct AddHeader<E> {
            ep: E,
            header: HeaderName,
            value: HeaderValue,
        }

        impl<E: Endpoint> Endpoint for AddHeader<E> {
            type Output = Response;

            async fn call(&self, req: Request) -> Result<Self::Output> {
                let mut resp = self.ep.call(req).await?.into_response();
                resp.headers_mut()
                    .insert(self.header.clone(), self.value.clone());
                Ok(resp)
            }
        }

        let ep = index.with(make(|ep| AddHeader {
            ep,
            header: HeaderName::from_static("hello"),
            value: HeaderValue::from_static("world"),
        }));
        let cli = TestClient::new(ep);

        let resp = cli.get("/").send().await;
        resp.assert_header("hello", "world");
        resp.assert_text("abc").await;
    }

    #[tokio::test]
    async fn test_with_multiple_middlewares() {
        #[handler(internal)]
        fn index(data: Data<&i32>) -> String {
            data.0.to_string()
        }

        let ep = index.with((
            AddData::new(10),
            SetHeader::new().appending("myheader-1", "a"),
            SetHeader::new().appending("myheader-2", "b"),
        ));
        let cli = TestClient::new(ep);

        let resp = cli.get("/").send().await;
        resp.assert_status_is_ok();
        resp.assert_header("myheader-1", "a");
        resp.assert_header("myheader-2", "b");
        resp.assert_text("10").await;
    }
}
