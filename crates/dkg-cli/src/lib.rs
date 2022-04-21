pub mod actions;
mod dkg_contract;
pub mod opts;

use async_trait::async_trait;
use dkg_contract::DKG;
use ethers::{contract::ContractError, prelude::Middleware, providers::ProviderError};

use dkg_core::{
    primitives::{BundledJustification, BundledResponses, BundledShares},
    BoardPublisher,
};
use thiserror::Error;
use threshold_bls::group::Curve;

#[derive(Debug, Error)]
pub enum DKGContractError<M: Middleware> {
    #[error(transparent)]
    SerializationError(#[from] bincode::Error),
    #[error(transparent)]
    PublishingError(#[from] ContractError<M>),
    #[error(transparent)]
    ProviderError(#[from] ProviderError),
}

#[async_trait]
impl<C: Curve, M: ethers::providers::Middleware> BoardPublisher<C> for DKG<M> {
    type Error = DKGContractError<M>;

    async fn publish_shares(&mut self, shares: BundledShares<C>) -> Result<(), Self::Error>
    where
        C: 'async_trait,
    {
        let serialized = bincode::serialize(&shares)?;
        let tx = self.publish(serialized);
        let pending_tx = tx.send().await?;
        let _tx_receipt = pending_tx.confirmations(6).await?; //self.pending_transaction(pending_tx).await?;
        Ok(())
    }

    async fn publish_responses(&mut self, responses: BundledResponses) -> Result<(), Self::Error>
    where
        C: 'async_trait,
    {
        let serialized = bincode::serialize(&responses)?;
        let tx = self.publish(serialized);
        let pending_tx = tx.send().await?;
        let _tx_receipt = pending_tx.confirmations(6).await?;
        Ok(())
    }

    async fn publish_justifications(
        &mut self,
        justifications: BundledJustification<C>,
    ) -> Result<(), Self::Error>
    where
        C: 'async_trait,
    {
        let serialized = bincode::serialize(&justifications)?;
        let tx = self.publish(serialized);
        let pending_tx = tx.send().await?;
        let _tx_receipt = pending_tx.confirmations(6).await?;
        Ok(())
    }
}
