const mocks = {
    mocksUser: {
        "Type": 0,
        "CreateFee": 1,
        "UpdateFee": 1,
        "MaxSize": 1000,
        "MaxCount": 1,
        "PruneDepth": 0,
        "RateTrigger": 0,
        "Header": {
            "RegTX": "<txid>",
            "AccKey": "pierre",
            "ObjNonce": 1,
            "TreeIDX": 0,
            "DataHash": "string",
            "Relations": null,
            "Sig": "<signature of header properties>"
        },

        "Data": {
            "Blobs": {
                "HDRootPubKey": "string",
                "BlockedUsers": ["string", "string"]
            },
            "Summary": "string",
            "ImgURL": "string"
        },

        "BanParticipation": 0,
        "BanMajority": 0,

        "State": {
            "Rating": 0,
            "Balance": 0,
            "Status": 0
        }
    },
    mnList: [
        { id: 'Masternode_0', PubKey: '4501b8e705efb73cf52cd040d26f3f1d78f76dd80ff02b0a4019486f7eabdf92' },
        { id: 'Masternode_1', PubKey: '282a8b83d2e7e6075c11f5e5ff9eca6a43cee8573dc90693a99cbeeb6bf385bd' },
        { id: 'Masternode_2', PubKey: '9b2e91a1b5ac68de45f749ad037e8c69ee417d2b0709988f6ddf2ca434f3a293' },
        { id: 'Masternode_3', PubKey: '7d1f41211710159c0acd34be26abd695d03bf8dabcbdcb2fd3fdea42f67a4b4f' },
        { id: 'Masternode_4', PubKey: '1f4d166e31eebf1db53e3564cbf1bdba8afe6bdb11d2cfb6d52f40b2fa65693b' },
        { id: 'Masternode_5', PubKey: '9c275f411927d5e82bae5def80034664a4b1e5b4d199da109ada531deb2f32fb' },
        { id: 'Masternode_6', PubKey: 'f445df03267eeb4bdcf359f6c1ce894dbd0dd035ef94b68219ee991b4adbb978' },
        { id: 'Masternode_7', PubKey: 'a86b1bf86330cba53fb2083c99a3f3a883b07f244373f2629b359f129235a98f' },
        { id: 'Masternode_8', PubKey: 'ff0ae33eef11fc07d5a073c3d98bea160e68c4770a394995c2011e11157da034' },
        { id: 'Masternode_9', PubKey: 'c7e95aac5fd20170404ebb62a6bdf8721954d801d85cf2a58ae492b1a9601801' }
    ]
};

module.exports = mocks;