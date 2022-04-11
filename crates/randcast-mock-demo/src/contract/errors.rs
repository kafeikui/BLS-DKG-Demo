use thiserror::Error;

pub type ControllerResult<A> = Result<A, ControllerError>;

#[derive(Debug, Error)]
pub enum ControllerError {
    #[error("signature task not found in list of pending_signature_tasks")]
    TaskNotFound,

    #[error("signature task is still exclusive for the assigned group")]
    TaskStillExclusive,

    #[error("signature task can only be fulfilled by the committer")]
    NotFromCommitter,

    #[error("the group index is not exist")]
    GroupNotExisted,

    #[error("the node is not registered")]
    NodeNotExisted,

    #[error("the node has already registered")]
    NodeExisted,

    #[error("the node is ready to work")]
    NodeActivated,

    #[error("the node is pending until block height #{0}")]
    NodeNotAvailable(usize),

    #[error("the reward record of the address is not exist")]
    RewardRecordNotExisted,

    #[error("the group index is different from the latest: {0}")]
    GroupEpochObsolete(usize),

    #[error("you have already committed the dkg output")]
    CommitCacheExisted,

    #[error("there is pending verifiable_signature_reward related to the node as the committer")]
    VerifiableSignatureRewardAsCommitterExisted,

    #[error("the verifiable_signature_reward is not exist")]
    VerifiableSignatureRewardNotExisted,

    #[error("the verifiable_signature_reward has been verified successfully")]
    SignatureRewardVerifiedSuccessfully,

    #[error("deserialization failed: the public key is not a valid G1 point {0})")]
    PublicKeyBadFormat(#[from] bincode::Error),

    #[error("the participant is not in the specified group")]
    ParticipantNotExisted,

    #[error("there is no valid group to generate randomness for now")]
    NoVaildGroup,
}
