/**
 * Test data and fixtures for WASM SDK tests
 */

const TestData = {
    // Valid test identities (from testnet)
    identities: {
        valid: '5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk',
        another: '3mFKtDYspCMd8YmXNTB3qzKmbY3Azf4Kx3x8e36V8Gho',
        empty: 'EmptyIdentityForTestingPurposes123456789ABCDef'
    },
    
    // Valid test contracts
    contracts: {
        dpns: 'GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec',
        dashpay: 'ABC123DEF456GHI789JKL012MNO345PQR678STU901VWX',
        custom: 'CustomContractId123456789ABCDEF0123456789ABC'
    },
    
    // Test seed phrases
    seeds: {
        // Standard test seed (DO NOT use with real funds)
        test: "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
        // Alternative test seeds
        test2: "test test test test test test test test test test test junk",
        test3: "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong"
    },
    
    // Test derivation paths
    paths: {
        bip44: {
            mainnet: "m/44'/5'/0'/0/0",
            testnet: "m/44'/1'/0'/0/0"
        },
        dip13: {
            auth: "m/9'/5'/5'/0'/0'/0'/0'",
            registration: "m/9'/5'/5'/1'/0'/0'/0'",
            topup: "m/9'/5'/5'/2'/0'/0'/0'"
        },
        dip15: {
            contact: "m/9'/5'/15'/0'/SENDER_ID/RECEIVER_ID/0"
        }
    },
    
    // Test addresses
    addresses: {
        mainnet: {
            valid: ["Xj8MfkgKGqGhRfXfkrBUGBqNXv7YjqrKZ8", "XoC7RfZxYpgwf6nKYRs7xQ8V9mGcKDuN2P"],
            invalid: ["invalid-address", "123", ""]
        },
        testnet: {
            valid: ["yTYFoEGfsnhVNekgHMWVWgx6VXyq8Yb5mp", "yXxJVKrRMoKjhyBN3QYyVQYjhgKmNPQx4g"],
            invalid: ["invalid-testnet", "xyz", ""]
        }
    },
    
    // Common test parameters
    params: {
        timeout: 5000,
        retries: 3,
        limits: {
            small: 10,
            medium: 50,
            large: 100
        }
    },
    
    // Mock network responses
    mocks: {
        identity: {
            balance: 1000000,
            nonce: 42,
            revision: 1,
            keys: [
                {
                    id: 0,
                    type: "ECDSA_SECP256K1",
                    purpose: "AUTHENTICATION",
                    data: "A123B456C789"
                }
            ]
        },
        
        document: {
            id: "DocumentId123456789ABCDEF0123456789ABCDEF0",
            type: "note",
            ownerId: "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk",
            data: {
                message: "Hello World"
            }
        },
        
        contract: {
            id: "ContractId123456789ABCDEF0123456789ABCDEF0",
            version: 1,
            ownerId: "5DbLwAxGBzUzo81VewMUwn4b5P4bpv9FNFybi25XB5Bk",
            schema: {
                note: {
                    type: "object",
                    properties: {
                        message: { type: "string" }
                    }
                }
            }
        }
    }
};

module.exports = { TestData };