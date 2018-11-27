const mocks = {
  mocksUser: {
    Type: 0,
    CreateFee: 1,
    UpdateFee: 1,
    MaxSize: 1000,
    MaxCount: 1,
    PruneDepth: 0,
    RateTrigger: 0,
    Header: {
      RegTX: '<txid>',
      AccKey: 'pierre',
      ObjNonce: 1,
      TreeIDX: 0,
      DataHash: 'string',
      Relations: null,
      Sig: '<signature of header properties>',
    },

    Data: {
      Blobs: {
        HDRootPubKey: 'string',
        BlockedUsers: ['string', 'string'],
      },
      Summary: 'string',
      ImgURL: 'string',
    },

    BanParticipation: 0,
    BanMajority: 0,

    State: {
      Rating: 0,
      Balance: 0,
      Status: 0,
    },
  },
  mnList: [{
    id: 'MasterNode_0',
    privKey: '16d5bd5088656193913414cc9141394bc38b02d717f0d2b55dcaeebe00adc8f3',
    publicAdr: 'Xc7HnuA5t9x5Gyvw5QfvKAcPpMDbjwtPch',
  },
  {
    id: 'MasterNode_1',
    privKey: 'a593a85cb8e1ddcfcc389a1b6220c2da9c8be23044ab9700a075e77b12e45b38',
    publicAdr: 'XduvHodMDGeUFwUdVqX93zihgPfxvHLCXN',
  },
  {
    id: 'MasterNode_2',
    privKey: 'bcdae83c99b77f230bc98fc73af5f4d886b294fcd7eccd578f4981bb0f2f8b3a',
    publicAdr: 'XjCNThZyPfGUb7dwkQsA77tHoG3QbUcbte',
  },
  {
    id: 'MasterNode_3',
    privKey: 'decfa6906d6ae148e86ea27bdc9d93b1fbafaf88812f7d1d723f342af82f8ad0',
    publicAdr: 'XpWT1u9NWXMHnxLAUy9oia5UHmppB5be2A',
  },
  {
    id: 'MasterNode_4',
    privKey: 'bd3b2904f8be0e43f0c34807457038f0c8312e214a89a0c14804826af90f8b0f',
    publicAdr: 'Xodhk14TsEgsxNdEJUEGQBoAmerv74TfzT',
  },
  {
    id: 'MasterNode_5',
    privKey: '0830081b2f685d05663d756155c5b78573f24fbc7bd81e91cd25272e78a10bd8',
    publicAdr: 'XnvG8dvwg7QH6HgPcKxiJFZthNKWjAy4Rz',
  },
  {
    id: 'MasterNode_6',
    privKey: 'acffcc3cd2d3503f2acc66136ffba7ef4f5afcd0c8e6470a667fdd8ee638d49c',
    publicAdr: 'XwK32NqbXGpNK8Utsg9sZjcPjhNxyVGRM7',
  },
  {
    id: 'MasterNode_7',
    privKey: '167174feb02956ea149d78e0f3667e0a9676755be0cebb3095d03131971051',
    publicAdr: 'Xj6v64MpMNtYhKPrRJK5x9hibdRiXefZr5',
  },
  {
    id: 'MasterNode_8',
    privKey: 'be46751b9883001bc172a11754f1b5f35563b7fd3bf1a481ead341fd901ff4e9',
    publicAdr: 'XjJzGkP8juDyPGJ3zJNHftzu4mKvneuFSZ',
  },
  {
    id: 'MasterNode_9',
    privKey: '95bc48b247349aebeb6a17139aad31aa223d5823c0ae47e42e17002eb35a28c4',
    publicAdr: 'XprYm1Dd693SYpUoTCAPWZXTyhVZBrx54A',
  }],
};

module.exports = mocks;
