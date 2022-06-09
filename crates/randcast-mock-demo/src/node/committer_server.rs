use self::committer::{
    committer_service_server::{CommitterService, CommitterServiceServer},
    CommitPartialSignatureReply, CommitPartialSignatureRequest,
};
use super::{
    bls::{BLSCore, MockBLSCore},
    cache::{
        GroupInfoFetcher, InMemoryGroupInfoCache, InMemorySignatureResultCache,
        SignatureResultCacheFetcher, SignatureResultCacheUpdater,
    },
    errors::NodeError,
};
use futures::Future;
use parking_lot::RwLock;
use std::sync::Arc;
use tonic::{transport::Server, Request, Response, Status};

pub mod committer {
    include!("../../stub/committer.rs");
}

pub struct BLSCommitterServiceServer {
    group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
    committer_cache: Arc<RwLock<InMemorySignatureResultCache>>,
}

impl BLSCommitterServiceServer {
    pub fn new(
        group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
        committer_cache: Arc<RwLock<InMemorySignatureResultCache>>,
    ) -> Self {
        BLSCommitterServiceServer {
            group_cache,
            committer_cache,
        }
    }
}

#[tonic::async_trait]
impl CommitterService for BLSCommitterServiceServer {
    async fn commit_partial_signature(
        &self,
        request: Request<CommitPartialSignatureRequest>,
    ) -> Result<Response<CommitPartialSignatureReply>, Status> {
        let req = request.into_inner();

        if let Ok(member) = self.group_cache.read().get_member(&req.id_address) {
            if !self.group_cache.read().get_state().unwrap() {
                return Err(Status::not_found(NodeError::GroupNotReady.to_string()));
            }

            let partial_public_key = member.partial_public_key.unwrap();

            let bls_core = MockBLSCore {};

            bls_core
                .partial_verify(&partial_public_key, &req.message, &req.partial_signature)
                .map_err(|e| Status::internal(e.to_string()))?;

            if !self
                .committer_cache
                .read()
                .contains(req.signature_index as usize)
            {
                let group_index = self
                    .group_cache
                    .read()
                    .get_index()
                    .map_err(|e| Status::internal(e.to_string()))?;

                let threshold = self
                    .group_cache
                    .read()
                    .get_threshold()
                    .map_err(|e| Status::internal(e.to_string()))?;

                self.committer_cache
                    .write()
                    .add(group_index, req.signature_index as usize, threshold)
                    .map_err(|e| Status::internal(e.to_string()))?;
            }

            self.committer_cache
                .write()
                .add_partial_signature(
                    req.signature_index as usize,
                    req.id_address,
                    req.partial_signature,
                )
                .unwrap();

            return Ok(Response::new(CommitPartialSignatureReply { result: true }));
        }

        Err(Status::not_found(NodeError::MemberNotExisted.to_string()))
    }
}

pub async fn start_committer_server<F: Future<Output = ()>>(
    endpoint: String,
    group_cache: Arc<RwLock<InMemoryGroupInfoCache>>,
    committer_cache: Arc<RwLock<InMemorySignatureResultCache>>,
    shutdown_signal: F,
) -> Result<(), Box<dyn std::error::Error>> {
    let addr = endpoint.parse()?;

    Server::builder()
        .add_service(CommitterServiceServer::with_interceptor(
            BLSCommitterServiceServer::new(group_cache, committer_cache),
            intercept,
        ))
        .serve_with_shutdown(addr, shutdown_signal)
        .await?;
    Ok(())
}

fn intercept(req: Request<()>) -> Result<Request<()>, Status> {
    // println!("Intercepting request: {:?}", req);

    Ok(req)
}
