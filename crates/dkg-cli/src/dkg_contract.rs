pub use dkg_mod::*;
#[allow(clippy::too_many_arguments)]
mod dkg_mod {
    #![allow(clippy::enum_variant_names)]
    #![allow(dead_code)]
    #![allow(clippy::type_complexity)]
    #![allow(unused_imports)]
    use ethers::contract::{
        builders::{ContractCall, Event},
        Contract, Lazy,
    };
    use ethers::core::{
        abi::{Abi, Detokenize, InvalidOutputType, Token, Tokenizable},
        types::*,
    };
    use ethers::providers::Middleware;
    #[doc = "DKG was auto-generated with ethers-rs Abigen. More information at: https://github.com/gakonst/ethers-rs"]
    use std::sync::Arc;
    pub static DKG_ABI: ethers::contract::Lazy<ethers::core::abi::Abi> =
        ethers::contract::Lazy::new(|| {
            serde_json :: from_str ("[{\"inputs\":[{\"internalType\":\"uint256\",\"name\":\"threshold\",\"type\":\"uint256\"},{\"internalType\":\"uint256\",\"name\":\"duration\",\"type\":\"uint256\"}],\"stateMutability\":\"nonpayable\",\"type\":\"constructor\"},{\"inputs\":[],\"name\":\"PHASE_DURATION\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\"}],\"stateMutability\":\"view\",\"type\":\"function\"},{\"inputs\":[],\"name\":\"THRESHOLD\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\"}],\"stateMutability\":\"view\",\"type\":\"function\"},{\"inputs\":[{\"internalType\":\"address\",\"name\":\"user\",\"type\":\"address\"}],\"name\":\"allowlist\",\"outputs\":[],\"stateMutability\":\"nonpayable\",\"type\":\"function\"},{\"inputs\":[],\"name\":\"getBlsKeys\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\"},{\"internalType\":\"bytes[]\",\"name\":\"\",\"type\":\"bytes[]\"}],\"stateMutability\":\"view\",\"type\":\"function\"},{\"inputs\":[],\"name\":\"getJustifications\",\"outputs\":[{\"internalType\":\"bytes[]\",\"name\":\"\",\"type\":\"bytes[]\"}],\"stateMutability\":\"view\",\"type\":\"function\"},{\"inputs\":[],\"name\":\"getParticipants\",\"outputs\":[{\"internalType\":\"address[]\",\"name\":\"\",\"type\":\"address[]\"}],\"stateMutability\":\"view\",\"type\":\"function\"},{\"inputs\":[],\"name\":\"getResponses\",\"outputs\":[{\"internalType\":\"bytes[]\",\"name\":\"\",\"type\":\"bytes[]\"}],\"stateMutability\":\"view\",\"type\":\"function\"},{\"inputs\":[],\"name\":\"getShares\",\"outputs\":[{\"internalType\":\"bytes[]\",\"name\":\"\",\"type\":\"bytes[]\"}],\"stateMutability\":\"view\",\"type\":\"function\"},{\"inputs\":[],\"name\":\"inPhase\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\"}],\"stateMutability\":\"view\",\"type\":\"function\"},{\"inputs\":[{\"internalType\":\"address\",\"name\":\"\",\"type\":\"address\"}],\"name\":\"justifications\",\"outputs\":[{\"internalType\":\"bytes\",\"name\":\"\",\"type\":\"bytes\"}],\"stateMutability\":\"view\",\"type\":\"function\"},{\"inputs\":[{\"internalType\":\"address\",\"name\":\"\",\"type\":\"address\"}],\"name\":\"keys\",\"outputs\":[{\"internalType\":\"bytes\",\"name\":\"\",\"type\":\"bytes\"}],\"stateMutability\":\"view\",\"type\":\"function\"},{\"inputs\":[],\"name\":\"owner\",\"outputs\":[{\"internalType\":\"address\",\"name\":\"\",\"type\":\"address\"}],\"stateMutability\":\"view\",\"type\":\"function\"},{\"inputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\"}],\"name\":\"participants\",\"outputs\":[{\"internalType\":\"address\",\"name\":\"\",\"type\":\"address\"}],\"stateMutability\":\"view\",\"type\":\"function\"},{\"inputs\":[{\"internalType\":\"bytes\",\"name\":\"value\",\"type\":\"bytes\"}],\"name\":\"publish\",\"outputs\":[],\"stateMutability\":\"nonpayable\",\"type\":\"function\"},{\"inputs\":[{\"internalType\":\"bytes\",\"name\":\"blsPublicKey\",\"type\":\"bytes\"}],\"name\":\"register\",\"outputs\":[],\"stateMutability\":\"nonpayable\",\"type\":\"function\"},{\"inputs\":[{\"internalType\":\"address\",\"name\":\"\",\"type\":\"address\"}],\"name\":\"responses\",\"outputs\":[{\"internalType\":\"bytes\",\"name\":\"\",\"type\":\"bytes\"}],\"stateMutability\":\"view\",\"type\":\"function\"},{\"inputs\":[{\"internalType\":\"address\",\"name\":\"\",\"type\":\"address\"}],\"name\":\"shares\",\"outputs\":[{\"internalType\":\"bytes\",\"name\":\"\",\"type\":\"bytes\"}],\"stateMutability\":\"view\",\"type\":\"function\"},{\"inputs\":[],\"name\":\"start\",\"outputs\":[],\"stateMutability\":\"nonpayable\",\"type\":\"function\"},{\"inputs\":[],\"name\":\"startBlock\",\"outputs\":[{\"internalType\":\"uint256\",\"name\":\"\",\"type\":\"uint256\"}],\"stateMutability\":\"view\",\"type\":\"function\"},{\"inputs\":[{\"internalType\":\"address\",\"name\":\"\",\"type\":\"address\"}],\"name\":\"userState\",\"outputs\":[{\"internalType\":\"enum DKG.UserState\",\"name\":\"\",\"type\":\"uint8\"}],\"stateMutability\":\"view\",\"type\":\"function\"}]") . expect ("invalid abi")
        });
    #[derive(Clone)]
    pub struct DKG<M>(ethers::contract::Contract<M>);
    impl<M> std::ops::Deref for DKG<M> {
        type Target = ethers::contract::Contract<M>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<M: ethers::providers::Middleware> std::fmt::Debug for DKG<M> {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.debug_tuple(stringify!(DKG))
                .field(&self.address())
                .finish()
        }
    }
    impl<'a, M: ethers::providers::Middleware> DKG<M> {
        #[doc = r" Creates a new contract instance with the specified `ethers`"]
        #[doc = r" client at the given `Address`. The contract derefs to a `ethers::Contract`"]
        #[doc = r" object"]
        pub fn new<T: Into<ethers::core::types::Address>>(
            address: T,
            client: ::std::sync::Arc<M>,
        ) -> Self {
            let contract = ethers::contract::Contract::new(address.into(), DKG_ABI.clone(), client);
            Self(contract)
        }
        #[doc = "Calls the contract's `PHASE_DURATION` (0x4ae2b849) function"]
        pub fn phase_duration(
            &self,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::U256> {
            self.0
                .method_hash([74, 226, 184, 73], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `THRESHOLD` (0x785ffb37) function"]
        pub fn threshold(
            &self,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::U256> {
            self.0
                .method_hash([120, 95, 251, 55], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `allowlist` (0xa7cd52cb) function"]
        pub fn allowlist(
            &self,
            user: ethers::core::types::Address,
        ) -> ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([167, 205, 82, 203], user)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `getBlsKeys` (0xa8194596) function"]
        pub fn get_bls_keys(
            &self,
        ) -> ethers::contract::builders::ContractCall<M, (ethers::core::types::U256, Vec<Vec<u8>>)>
        {
            self.0
                .method_hash([168, 25, 69, 150], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `getJustifications` (0xb0ef8179) function"]
        pub fn get_justifications(
            &self,
        ) -> ethers::contract::builders::ContractCall<M, Vec<Vec<u8>>> {
            self.0
                .method_hash([176, 239, 129, 121], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `getParticipants` (0x5aa68ac0) function"]
        pub fn get_participants(
            &self,
        ) -> ethers::contract::builders::ContractCall<M, Vec<ethers::core::types::Address>>
        {
            self.0
                .method_hash([90, 166, 138, 192], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `getResponses` (0xcc5ef009) function"]
        pub fn get_responses(&self) -> ethers::contract::builders::ContractCall<M, Vec<Vec<u8>>> {
            self.0
                .method_hash([204, 94, 240, 9], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `getShares` (0xd73fe0aa) function"]
        pub fn get_shares(&self) -> ethers::contract::builders::ContractCall<M, Vec<Vec<u8>>> {
            self.0
                .method_hash([215, 63, 224, 170], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `inPhase` (0x221f9511) function"]
        pub fn in_phase(
            &self,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::U256> {
            self.0
                .method_hash([34, 31, 149, 17], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `justifications` (0xcd5e3837) function"]
        pub fn justifications(
            &self,
            p0: ethers::core::types::Address,
        ) -> ethers::contract::builders::ContractCall<M, Vec<u8>> {
            self.0
                .method_hash([205, 94, 56, 55], p0)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `keys` (0x670d14b2) function"]
        pub fn keys(
            &self,
            p0: ethers::core::types::Address,
        ) -> ethers::contract::builders::ContractCall<M, Vec<u8>> {
            self.0
                .method_hash([103, 13, 20, 178], p0)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `owner` (0x8da5cb5b) function"]
        pub fn owner(
            &self,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::Address> {
            self.0
                .method_hash([141, 165, 203, 91], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `participants` (0x35c1d349) function"]
        pub fn participants(
            &self,
            p0: ethers::core::types::U256,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::Address> {
            self.0
                .method_hash([53, 193, 211, 73], p0)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `publish` (0x7fd28346) function"]
        pub fn publish(&self, value: Vec<u8>) -> ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([127, 210, 131, 70], value)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `register` (0x82fbdc9c) function"]
        pub fn register(
            &self,
            bls_public_key: Vec<u8>,
        ) -> ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([130, 251, 220, 156], bls_public_key)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `responses` (0x0ea65648) function"]
        pub fn responses(
            &self,
            p0: ethers::core::types::Address,
        ) -> ethers::contract::builders::ContractCall<M, Vec<u8>> {
            self.0
                .method_hash([14, 166, 86, 72], p0)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `shares` (0xce7c2ac2) function"]
        pub fn shares(
            &self,
            p0: ethers::core::types::Address,
        ) -> ethers::contract::builders::ContractCall<M, Vec<u8>> {
            self.0
                .method_hash([206, 124, 42, 194], p0)
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `start` (0xbe9a6555) function"]
        pub fn start(&self) -> ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([190, 154, 101, 85], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `startBlock` (0x48cd4cb1) function"]
        pub fn start_block(
            &self,
        ) -> ethers::contract::builders::ContractCall<M, ethers::core::types::U256> {
            self.0
                .method_hash([72, 205, 76, 177], ())
                .expect("method not found (this should never happen)")
        }
        #[doc = "Calls the contract's `userState` (0x0c8f81b5) function"]
        pub fn user_state(
            &self,
            p0: ethers::core::types::Address,
        ) -> ethers::contract::builders::ContractCall<M, u8> {
            self.0
                .method_hash([12, 143, 129, 181], p0)
                .expect("method not found (this should never happen)")
        }
    }
}
