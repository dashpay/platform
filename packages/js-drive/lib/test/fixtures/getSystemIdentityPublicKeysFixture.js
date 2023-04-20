const { PublicKey } = require('@dashevo/dashcore-lib');

function getSystemIdentityPublicKeysFixture() {
  return {
    masternodeRewardSharesContractOwner: {
      master: new PublicKey(process.env.MASTERNODE_REWARD_SHARES_MASTER_PUBLIC_KEY).toBuffer(),
      high: new PublicKey(process.env.MASTERNODE_REWARD_SHARES_SECOND_PUBLIC_KEY).toBuffer(),
    },
    featureFlagsContractOwner: {
      master: new PublicKey(process.env.FEATURE_FLAGS_MASTER_PUBLIC_KEY).toBuffer(),
      high: new PublicKey(process.env.FEATURE_FLAGS_SECOND_PUBLIC_KEY).toBuffer(),
    },
    dpnsContractOwner: {
      master: new PublicKey(process.env.DPNS_MASTER_PUBLIC_KEY).toBuffer(),
      high: new PublicKey(process.env.DPNS_SECOND_PUBLIC_KEY).toBuffer(),
    },
    withdrawalsContractOwner: {
      master: new PublicKey(process.env.WITHDRAWALS_MASTER_PUBLIC_KEY).toBuffer(),
      high: new PublicKey(process.env.WITHDRAWALS_SECOND_PUBLIC_KEY).toBuffer(),
    },
    dashpayContractOwner: {
      master: new PublicKey(process.env.DASHPAY_MASTER_PUBLIC_KEY).toBuffer(),
      high: new PublicKey(process.env.DASHPAY_SECOND_PUBLIC_KEY).toBuffer(),
    },
  };
}

module.exports = getSystemIdentityPublicKeysFixture;
