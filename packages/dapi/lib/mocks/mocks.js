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
    }
};

module.exports = mocks;