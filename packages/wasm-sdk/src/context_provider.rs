use std::sync::Arc;

use dash_sdk::{
    dpp::{
        prelude::CoreBlockHeight,
        util::vec::{decode_hex, encode_hex},
    },
    error::ContextProviderError,
    platform::{DataContract, Identifier},
};
use drive_proof_verifier::ContextProvider;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub struct WasmContext {}
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
    "0000000000000028113a56cdec51df03d0a4786886455b56c405888820ef0941:a9cc0b5debb51078cc74666ce230512b5cc8a210e92e7a61143a94f2d39e0f2396fdb8a236f9e4f08bb9f9efade56aa0",
    "000000000000001ca970d7cd594bba828499c114e7c414ffd76ca2472dbb628c:878251f1d4c7d126877159edf7393f1bc7f7fa8cd6a90b4b533bd464ba89c7439fd39e405a3ef9bb629c46a2150826f0",
    "0000000000000008a01171418cdd051431ee153871a919a66c2c0ad0c879cf26:9919eaf780965b8ca0ec77b53c50b72cc382b3e8411c347f9f08c7285d77c44ba7fa07a023258b15f024fecf6332c5b7",
    "0000000000000019d45a8e21c319cc0697798a5fb3beaf613d33bf960d6a6cbd:8132c57180bcfa1849837b28688c89e0e9547c637f8da62d7f5f48b6ad2f51ff3fc9466040b5a69e956a6814e09952ae",
    "0000000000000005b46622b4f786df8170e3c72d56642d0959acc76f4798bfd6:8d75addd7d32795eaeeb63c65a15c4f61062e340cfcac56d915f387500714bb70db51791d2e9a6b3435e3dacb60ba98e",
    "0000000000000001fce9c7c2a8b0f12827dad4b7e4748425ccb55ce21dd2bb1b:8c2d30a0becc4bc12b8cdcaea76c0b86b66cc1fce837acdc0f213af4c6ea2fb6606f65bd640316803b9c68e3c7228910",
    "000000000000001abb166439088af6adefea719f6db41239bf336a00145fac4b:b4e1cb36f6d8fe47194aa6d266e1fcc31396b68a2cb48c55c9e6503933bfd699683e90f0a468df3ff037405038597a28",
    "0000000000000014e8b2bfe48594251aab8f21db05d8ae8a87e071e8fc3e3cfb:ac0d4951a1405850593a90ac0f74049778b89a547b98aafbe1dd4685e3274d6700f2d886990bdf69de7fdcd6dd1e5ee8",
    "000000000000000b11fd2c3cdb0a31bf6e8c6811f7f57ebc715dcc9fff4c619b:85bd11bb6eae1fdf8ba7373315ce4841354c295a052bad29c91f2c9b795e7ff14810651f474863b509e97a217e3cf3c5",
    "000000000000000729844808a669505fd8281e7e965eb441a036fe6cbeb4693f:83757a46d09089db700dc49acf3ad009330bb349d6ec9235cd92c0a397474bb44f225b5d2dfca6388ce6bdc2b96a5926",
    "0000000000000002440753acfe1a9a8632cd53324b21d7212362ba924a7b4cc8:ae108342f52fdf86ede01f6b815636337a05e824000071c60b26c98b9fc7d29fada982b4eaaedd61269372182b9cfc95",
    "000000000000000afc58484fac033caa1aa088c96c01d450ae1de8cd8dd38a6e:8c6dba5bde17750d929bbabd2ab1dc15773166d5bba79dafceff8e9a799e7c7492c0c798133b6ea574f0bb09b7c07a77",
    "000000000000001b84665ddc7527b20198e2462f103d397bdb02334c20a2be22:96d0333a2bf0db2e8a502104a1f0f8442648ff040c485f7da7d8a0703995f44e6759052b005ed64f49e97a5503ad88be",
    "000000000000001b9fd42c73a767c026d44dbf269da97f3688b28beb06cc986d:952752cc09be8d537fc8c43266e91b44d875b84c15741b054889b26c65f6f251742424839ef42c6fe74479630d14b160",
    "000000000000000a845c3966ddd0f2beb4742fe5879a4513b79dba8f19d12da0:937d433ed5df63877d59ec94fb096fa740cc517b9cec5253a6af911da394fcacf55bfeb9ac3930d0aa3fa5fe8248d9d0",
    "000000000000000ae3bf646448897ea0a322518901faaee8f2f1e02027f251d8:a662eb0b193880570455c1e7a189b46348e49a50becf6f07b2a34d2c50923e4875888be19e0a315421ccc92e47c7c3f4",
    "0000000000000003eb7db84bf0a0b8e526778844dcb5a10740dbe75f8fe72562:8d20289962877f6c8b4b1c2c0a9ca054ca1b41b4ae340544d74bac3223404fa7446a8aa116fa2befd37f1e50f3d28d53",
    "000000000000000ca431a523ba79b0a040e14af944b50ac68cc3cc9192e6518c:a3c16e1c558a1a9828b2550028f605a3ed6a519022c7a2e7723531e3e622f03759994980fafa9a193a0e784e3b331370",
    "00000000000000090aca1303c1455281cdf213107573b6887b28e7faf06b38a4:952520e764930f1c9c7a258a0689e67c9df9911e00777edf3d76b7914a0f0a0047cde86afc55888ead4b797fb8861dae",
    "0000000000000021768892dc6eda195c176e3b7afd3537927268e8e5357c44bd:98eaafbc3fda5dd44d61361385f6a1f7ccb228294a7fb2a87072c27959d492eaabff98c60d05df21ab28f4e4147f7d85",
    "0000000000000002aa4b5db3552bc3cd2fb52386ea84c713eca9cc11415e6a8e:8199ceaf1b719054432d52ae241819643f97af677b50f12a6c162f0124959597b0c68403421e971db14595a5dc3cf9be",
    "00000000000000022c52da255c3d388035a9c4baa0e1b4a3376ca3363eaf96d9:ab40da63160f496fd64e81a58cc45e5b21246f8a5dcbe1f782ae5e8bd30237ed4df5d984c7927db043bc623db8079635",
    "000000000000000c49614abc9f87d3122621e26933ebe82d17d94e6aa24f3797:87b5d5cd7a990a1cae0765af7447efeee90e7eb0b67736832c12442230004c5739d9ff8ae19bbac1658931b3136da816",
    "000000000000001f1342af01769448b2b4c635a02c420f4844ba9058b52c489e:b826904db02b0edb1798d800d9658677bcdab856e380350b2637254d7a01f022ece44c33a51f4f9fc313071fc0f7e670",
    ];

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
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        todo!()
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        todo!()
    }
}
