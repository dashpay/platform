use std::sync::Arc;

use dash_sdk::{
    dpp::{
        prelude::CoreBlockHeight,
        util::vec::{decode_hex, encode_hex},
        version::PlatformVersion,
        data_contract::TokenConfiguration,
    },
    error::ContextProviderError,
    platform::{DataContract, Identifier},
};
use dash_sdk::platform::ContextProvider;
use wasm_bindgen::prelude::wasm_bindgen;
use async_trait::async_trait;

#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmContext {}

/// A wrapper for TrustedHttpContextProvider that works in WASM
pub struct WasmTrustedContext {
    inner: rs_sdk_trusted_context_provider::TrustedHttpContextProvider,
}
/// Quorum keys for the testnet
/// This is a hardcoded list of quorum keys for the testnet.
/// This list was generated using the following script:
/// ```bash
// #!/bin/bash
/// function dash-cli() {
///   docker exec -ti dashmate_2d59c0c6_mainnet-core-1 dash-cli $@
/// }
///
///
/// # Get the list of all quorum hashes
/// quorum_hashes=$(dash-cli quorum list | jq -r '.llmq_100_67[]')
///
/// count=$(wc -w <<< $quorum_hashes)
///
/// echo "const QUORUM_KEYS: [&str; $count] = ["
/// # Iterate over each quorum hash and get the respective public key
/// for quorum_hash in $quorum_hashes; do
///     quorum_public_key=$(dash-cli quorum info 4 $quorum_hash | jq -r '.quorumPublicKey')
///     echo "\"$quorum_hash:$quorum_public_key\","
/// done
/// echo "];
/// ```
const QUORUM_KEYS: [&str; 24] = [
    "0000000000000010c832afa3f737b24aa0a19cb8c5fa8a7ebd51d195965c204e:a7123c10b0083c96665954dacbc92779b9bf5ee15b6e0b37de6c3ac6b5e611490611fbdf8128ea8012e1b8d79faf70e1",
    "000000000000000b7ef1903452e3a0234b893567fe4f0f093b0de655756e413d:a75bca441f781fde0ae048181ab64e5bc85a2541b25267148bced1d1cbc1ff223e9b0d767f06b4a0773b65a7f374f72d",
    "000000000000000aaf16a867b579ddbafc421ebf52b991c445aba9d42cc70f62:9665f284f989b998c053e4d34d022d7a9c1744214356150b6b815ba0c4b28dd6312ff3f6d86eb2ab49d84cb2a20518c5",
    "00000000000000097177f56b1d83fcaa96101cb3c5e5cc1e624422552265aab0:ab96113d539e30e7f8f1ec605ab9228b6825bef1ea55cdf66826bb8308edf92fa782db1c3c6c348fb2daf5e13422fdff",
    "00000000000000210518749e17c00b035a2a4982c906236c28c41ea2231bf7ef:803c3341a037217c6b8d6bf9a1f08b140749039fcbf2bd79d639ee6d70b16c9911f979e6f7014429d1a805876a9cb8dd",
    "000000000000000d9085a108ae45615da3fc0b43313959a7fde892bb307d4f6a:ab880781c7ef1b5a38202065a90afc92ff25009f231a16daa766528bf36b78856d2a9a35c4d396326df434eb09fb3897",
    "000000000000001871d3dab13ccefca7dd9735c27028e6612e8f1ac16114b080:ae5334bc41a2a2b1b52c8c454957f00a4554fb0ec82e7ca61b9391a4371346ab4679b55a15220af7ea657e153af867be",
    "00000000000000223578d6be39205e0cddd36a0c73cde66aee0a7ad4bf451931:986eb45b5cd503f6ae09070b93fbf101ac69999147cce55a2f1f4b04b2aaf0ad7ea57ea96ee3f4335bcdd13dce6577f6",
    "000000000000000208d59d5275610de33bca248e4055974573775affb2d99b0a:b4c419af96d4a053e90c6cc3be88c09f47543a8af9b5e94f0e345f91604843a60d8b19d045908b22ce52084f82f3a627",
    "00000000000000065644145a3a5f94f9786245f4242a09fadfe277de7059b44a:95bebdad569bfe6c870f6f96c6a5d265f2d8561997b2a8be2d3192ad16ee872b833593a8096d8d7b142ba6ba2f662147",
    "000000000000000ab82df09faf91a3e07b756dfc715c309a21feca21dd233559:91ebfc1415bf0e87e88024a1a794735130b8b438abdd29313f26bb3dece7577f77c80b0103a6c97c9a888f8e4334f44a",
    "0000000000000023b4287203df6f4e1e67726b4f9bbeeec96a3b2e47227bcf7c:b1cc46b8d00f193238550b035860994e95c1515d95ef675c890bf38257dfc44631cc1da786550c1df7bda83dea17299c",
    "000000000000000dd6431b32390b87c7c3c2377c46ecb8be2e36e0625400204b:a84f43efe056cd1221b9d4449dc3d0b3ce2e260f6378790f3e6123903d204ae83589e5a144533eb556acd05f920869a6",
    "0000000000000005609c7e47752ff16847a49bd81f12885d3047c26c1d4de394:8f08f63fdd26e3e15f4c6daf45943c6ae097a40f46f4344cea63bcdfa9498525cfe0891c9d04057692738fa8f5e7b30d",
    "000000000000001fb29b450739378551cd7539357e0ee49fd93c186eb8dd9db4:8cc74c3c7f10dd11845894dafafe47e60395d40bb151299638736dd078cc1052da13b3676e945d84cd4139a519a37975",
    "0000000000000005f3b003f2b70096157046005944b316950c2d6f31a7fbdb6c:82780a7c99e2e09589a42a38e7be3d78d53072b09d964762cd9025b5b0b88bbed7e9e6c294a4fc8310d5025668240eaa",
    "000000000000001f7bdf487d9ab675b9d4c53d8601b2b9dd619cf6f91271c1ef:ac79a22006a9c5d96250bd91784919a65259b5e1cf72af8294a998a420684ab3b9064094f247b0ebf1c5e39a545ac176",
    "00000000000000127ae21d0e85ab8f511309b3492a689e680872d05d3d5e27b9:887332518d8a6d8806dfb1dd9588d2d4a2ec80193b5357ed87fc49567a72752ec8022dfaf4f5dec22c85aff4139daeb3",
    "000000000000001c5d877e4b2b5964277c71377acd0e83d2b949426b54b761a2:a7149631eed0fdef466de0360ae09700b96fc29e76d46465efe60677596e7f93f8ff5e5e39917e04a3a26febe12b3b8a",
    "000000000000000dab4691b25d967682b1902c05eefe691312cf520f6f2a3063:acc45cbf3ead9a04347438a68d72f86ffec76211562786f86125bbc2a444026343149cfb3d21fd27b386b50094250099",
    "000000000000001aad53ced4755062f69421f7ca6c0bbe259e2626738c310759:8037c5282d443eb274a819a1d83758b29a97c12fdbbfa65a255e460045e71d9a5301f96ae76743787b12edf4f4e79e1c",
    "0000000000000003e9cc2a59ebbede397e73e951f0968d4c94e04a92edb5586f:a62305e5154688adcc129da4a11bd0da388a731e52a361896f27473056d4dccb3642526086f83f36accbcaea43468121",
    "000000000000001fd65c58acf87be67925c89348c5a76cc61cb467bbd9991a80:818283cbc34445f0c294d8ac071d4e43099fd318dad53337cd1daccc8a2aa760ee108c80f3f3c5bb01c41bba69ab2e8b",
    "0000000000000028113a56cdec51df03d0a4786886455b56c405888820ef0941:a9cc0b5debb51078cc74666ce230512b5cc8a210e92e7a61143a94f2d39e0f2396fdb8a236f9e4f08bb9f9efade56aa0",
];

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl ContextProvider for WasmContext {
    fn get_quorum_public_key(
        &self,
        _quorum_type: u32,
        quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        let quorum_label = encode_hex(&quorum_hash) + ":";
        let key_hex = QUORUM_KEYS
            .iter()
            .find(|key| key.starts_with(&quorum_label))
            .ok_or(ContextProviderError::InvalidQuorum(format!(
                "key for quorum {:?} not found in hardcoded dictionary",
                &quorum_label[0..quorum_label.len() - 1]
            )))?;
        let key = decode_hex(&key_hex[quorum_label.len()..])
            .map_err(|e| ContextProviderError::InvalidQuorum(e.to_string()))?
            .try_into()
            .map_err(|_e| {
                ContextProviderError::InvalidQuorum("invalid quorum key size".to_string())
            })?;

        Ok(key)
    }

    fn get_data_contract(
        &self,
        _id: &Identifier,
        _platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        // Return None for now - this means the contract will be fetched from the network
        Ok(None)
    }

    fn get_token_configuration(
        &self,
        _token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        // Return None for now - this means the token config will be fetched from the network
        Ok(None)
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        // Return a reasonable default for platform activation height
        // This is the height at which Platform was activated on testnet
        Ok(1)
    }

    async fn get_quorum_public_key_async(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        // For WASM, we can directly call the sync version since we're not doing any async operations
        self.get_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height)
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl ContextProvider for WasmTrustedContext {
    fn get_quorum_public_key(
        &self,
        _quorum_type: u32,
        _quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        // In WASM, we can't use the sync version
        Err(ContextProviderError::Generic(
            "Synchronous get_quorum_public_key not supported in WASM. Use async version.".to_string()
        ))
    }

    async fn get_quorum_public_key_async(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        // Use the async version from the inner provider
        self.inner.get_quorum_public_key_async(quorum_type, quorum_hash, core_chain_locked_height).await
    }

    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        self.inner.get_data_contract(id, platform_version)
    }

    fn get_token_configuration(
        &self,
        token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        self.inner.get_token_configuration(token_id)
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        self.inner.get_platform_activation_height()
    }
}

impl WasmTrustedContext {
    pub fn new_mainnet() -> Result<Self, ContextProviderError> {
        let inner = rs_sdk_trusted_context_provider::TrustedHttpContextProvider::new(
            dash_sdk::dpp::dashcore::Network::Dash,
            None,
            std::num::NonZeroUsize::new(100).unwrap(),
        ).map_err(|e| ContextProviderError::Generic(e.to_string()))?;
        
        Ok(Self { inner })
    }

    pub fn new_testnet() -> Result<Self, ContextProviderError> {
        let inner = rs_sdk_trusted_context_provider::TrustedHttpContextProvider::new(
            dash_sdk::dpp::dashcore::Network::Testnet,
            None,
            std::num::NonZeroUsize::new(100).unwrap(),
        ).map_err(|e| ContextProviderError::Generic(e.to_string()))?;
        
        Ok(Self { inner })
    }
}
