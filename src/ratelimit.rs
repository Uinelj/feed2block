use atrium_api::xrpc::{
    http::{Request, Response},
    HttpClient, XrpcClient,
};
use atrium_xrpc_client::reqwest::{ReqwestClient, ReqwestClientBuilder};
use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};
use std::{future::Future, num::NonZeroU32};

pub struct RateLimited<C: XrpcClient + HttpClient> {
    client: C,
    lim: DefaultDirectRateLimiter,
}

impl<C: XrpcClient + HttpClient> RateLimited<C> {
    pub fn new(client: C, lim: DefaultDirectRateLimiter) -> Self {
        Self { client, lim }
    }
}

impl RateLimited<ReqwestClient> {
    pub fn default_from_quota(quota: Quota) -> Self {
        Self::new(
            ReqwestClientBuilder::new("https://bsky.social").build(),
            RateLimiter::direct(quota),
        )
    }
}

impl Default for RateLimited<ReqwestClient> {
    fn default() -> Self {
        Self {
            client: ReqwestClientBuilder::new("https://bsky.social").build(),
            lim: RateLimiter::direct(Quota::per_second(NonZeroU32::try_from(10).unwrap())),
        }
    }
}

impl<C: HttpClient + XrpcClient + Sync> HttpClient for RateLimited<C> {
    #[doc = " Send an HTTP request and return the response."]
    fn send_http(
        &self,
        request: Request<Vec<u8>>,
    ) -> impl Future<
        Output = core::result::Result<
            Response<Vec<u8>>,
            Box<dyn std::error::Error + Send + Sync + 'static>,
        >,
    > + Send {
        Box::pin(async move {
            self.lim.until_ready().await;
            self.client.send_http(request).await
        })
    }
}

impl<C: XrpcClient + Sync> XrpcClient for RateLimited<C> {
    #[doc = " The base URI of the XRPC server."]
    fn base_uri(&self) -> String {
        self.client.base_uri()
    }
}
