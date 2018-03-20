/* eslint-disable */
// TODO: Make this file pass linting!
// created with /tests/User/create which uses the depricated way of OP_RETURN user registration

const mockUser = `{
    "txid": "89e3b3b4f62957ea94234293de9e01b3d509d5db67663c97d8369d018488bd12",
        "size": 325,
            "version": 1,
                "locktime": 0,
                    "vin": [
                        {
                            "txid": "0dc7155ca2d5cf251b4696dd513956d4ba0a4c202e6a0b1f8ffc721c61d1ced7",
                            "vout": 2,
                            "scriptSig": {
                                "asm": "3045022100aba732c100373171387d16da8df1d4ac5f3599777627faf4b05ed24736558cbe02204f505c25f8237117c1883212cc33f17de73428bddfbe623a30f08197eb3e6906[ALL] 026bfee0b8648b59d633b8b91e3cff4fc85e2137bcd895392c8e8f45b92e31aa28",
                                "hex": "483045022100aba732c100373171387d16da8df1d4ac5f3599777627faf4b05ed24736558cbe02204f505c25f8237117c1883212cc33f17de73428bddfbe623a30f08197eb3e69060121026bfee0b8648b59d633b8b91e3cff4fc85e2137bcd895392c8e8f45b92e31aa28"
                            },
                            "value": 999.993,
                            "valueSat": 99999300000,
                            "address": "yiBCPVWznF2nHDQD6H8wFWB8bhN8TKHFXc",
                            "sequence": 4294967295,
                            "n": 0,
                            "addr": "yiBCPVWznF2nHDQD6H8wFWB8bhN8TKHFXc",
                            "doubleSpentTxID": null,
                            "isConfirmed": true,
                            "confirmations": 7,
                            "unconfirmedInput": false
                        }
                    ],
                        "vout": [
                            {
                                "value": "0.00500000",
                                "valueSat": 500000,
                                "n": 0,
                                "scriptPubKey": {
                                    "asm": "OP_DUP OP_HASH160 745def4161800f792571c3fc03653db43b300e35 OP_EQUALVERIFY OP_CHECKSIG",
                                    "hex": "76a914745def4161800f792571c3fc03653db43b300e3588ac",
                                    "reqSigs": 1,
                                    "type": "pubkeyhash",
                                    "addresses": [
                                        "yWvjm9NkqGiRJjEC5xzZ5s2Yn1zS13wRkh"
                                    ]
                                }
                            },
                            {
                                "value": "0.00000000",
                                "valueSat": 0,
                                "n": 1,
                                "scriptPubKey": {
                                    "asm": "OP_RETURN 7b22616374696f6e223a22222c2274797065223a22222c226163634b6579223a22706965727265222c227075624b6579223a22586d4a386b434a4b506a344c787a4a655837674133716343566a5734537041667558227d",
                                    "hex": "6a4c577b22616374696f6e223a22222c2274797065223a22222c226163634b6579223a22706965727265222c227075624b6579223a22586d4a386b434a4b506a344c787a4a655837674133716343566a5734537041667558227d",
                                    "type": "nulldata"
                                }
                            },
                            {
                                "value": "999.98600000",
                                "valueSat": 99998600000,
                                "n": 2,
                                "scriptPubKey": {
                                    "asm": "OP_DUP OP_HASH160 efc35e569742bb002d032bc293b687f9ac4504ae OP_EQUALVERIFY OP_CHECKSIG",
                                    "hex": "76a914efc35e569742bb002d032bc293b687f9ac4504ae88ac",
                                    "reqSigs": 1,
                                    "type": "pubkeyhash",
                                    "addresses": [
                                        "yiBCPVWznF2nHDQD6H8wFWB8bhN8TKHFXc"
                                    ]
                                },
                                "spentTxId": "66b5ae8a56e303c9a8b576a46dcb0645f7f1f6e446a1cd3a3b3bcc2ab8e707af",
                                "spentIndex": 0,
                                "spentHeight": 7753,
                                "multipleSpentAttempts": [
                                    {
                                        "txid": "66b5ae8a56e303c9a8b576a46dcb0645f7f1f6e446a1cd3a3b3bcc2ab8e707af"
                                    },
                                    {
                                        "txid": "66b5ae8a56e303c9a8b576a46dcb0645f7f1f6e446a1cd3a3b3bcc2ab8e707af",
                                        "index": 0
                                    }
                                ]
                            }
                        ],
                            "blockhash": "00000000de7a4e61fce2ee71d50b15b29ad651fd13f9fb8257d12fa30dd9de61",
                                "height": 7749,
                                    "confirmations": 5,
                                        "time": 1507705066,
                                            "blocktime": 1507705066,
                                                "valueOut": 999.991,
                                                    "valueIn": 999.993,
                                                        "fees": 0.002
}`;

module.exports = mockUser;
