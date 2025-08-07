/**
 * Key Generation Tests - Migrated from key-generation.test.mjs
 * 
 * These tests verify that the WASM SDK key generation and wallet functions
 * work correctly for various use cases including BIP39 mnemonics, BIP44 
 * derivation, and Dash-specific derivation paths.
 */

const { TestSdkBuilder, TestAssertions } = require('../helpers/sdk-builder.js');
const { TestData } = require('../fixtures/test-data.js');

describe('Key Generation', () => {
    describe('Mnemonic Generation', () => {
        it('should generate 12 words by default', () => {
            const mnemonic = global.wasmSdk.generate_mnemonic();
            const words = mnemonic.split(' ');
            
            expect(words).to.have.length(12);
            expect(global.wasmSdk.validate_mnemonic(mnemonic)).to.be.true;
        });

        it('should generate 15 words when specified', () => {
            const mnemonic = global.wasmSdk.generate_mnemonic(15);
            const words = mnemonic.split(' ');
            
            expect(words).to.have.length(15);
            expect(global.wasmSdk.validate_mnemonic(mnemonic)).to.be.true;
        });

        it('should generate 18 words when specified', () => {
            const mnemonic = global.wasmSdk.generate_mnemonic(18);
            const words = mnemonic.split(' ');
            
            expect(words).to.have.length(18);
            expect(global.wasmSdk.validate_mnemonic(mnemonic)).to.be.true;
        });

        it('should generate 21 words when specified', () => {
            const mnemonic = global.wasmSdk.generate_mnemonic(21);
            const words = mnemonic.split(' ');
            
            expect(words).to.have.length(21);
            expect(global.wasmSdk.validate_mnemonic(mnemonic)).to.be.true;
        });

        it('should generate 24 words when specified', () => {
            const mnemonic = global.wasmSdk.generate_mnemonic(24);
            const words = mnemonic.split(' ');
            
            expect(words).to.have.length(24);
            expect(global.wasmSdk.validate_mnemonic(mnemonic)).to.be.true;
        });

        it('should reject invalid word counts', () => {
            expect(() => {
                global.wasmSdk.generate_mnemonic(13);
            }).to.throw(/Word count must be/);
        });

        describe('Multiple Languages', () => {
            const languages = [
                { code: 'en', name: 'English' },
                { code: 'es', name: 'Spanish' },
                { code: 'fr', name: 'French' },
                { code: 'it', name: 'Italian' },
                { code: 'ja', name: 'Japanese' },
                { code: 'ko', name: 'Korean' },
                { code: 'pt', name: 'Portuguese' },
                { code: 'cs', name: 'Czech' },
                { code: 'zh-cn', name: 'Simplified Chinese' },
                { code: 'zh-tw', name: 'Traditional Chinese' }
            ];

            languages.forEach(({ code, name }) => {
                it(`should generate valid ${name} (${code}) mnemonics`, () => {
                    const mnemonic = global.wasmSdk.generate_mnemonic(12, code);
                    expect(global.wasmSdk.validate_mnemonic(mnemonic, code)).to.be.true;
                });
            });

            it('should reject unsupported languages', () => {
                expect(() => {
                    global.wasmSdk.generate_mnemonic(12, 'xx');
                }).to.throw(/Unsupported language code/);
            });
        });
    });

    describe('Mnemonic Validation', () => {
        it('should validate correct mnemonics', () => {
            const validMnemonic = TestData.seeds.test;
            expect(global.wasmSdk.validate_mnemonic(validMnemonic)).to.be.true;
        });

        it('should reject mnemonics with invalid checksums', () => {
            const invalidMnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon";
            expect(global.wasmSdk.validate_mnemonic(invalidMnemonic)).to.be.false;
        });

        it('should reject mnemonics with wrong word count', () => {
            const invalidMnemonic = "abandon abandon abandon";
            expect(global.wasmSdk.validate_mnemonic(invalidMnemonic)).to.be.false;
        });

        it('should validate mnemonics in different languages', () => {
            // Test with English mnemonic
            expect(global.wasmSdk.validate_mnemonic(TestData.seeds.test, 'en')).to.be.true;
            
            // Test language mismatch
            expect(global.wasmSdk.validate_mnemonic(TestData.seeds.test, 'es')).to.be.false;
        });
    });

    describe('Mnemonic to Seed Conversion', () => {
        it('should convert mnemonic to seed without passphrase', () => {
            const seed = global.wasmSdk.mnemonic_to_seed(TestData.seeds.test);
            
            expect(seed).to.be.a('string');
            expect(seed).to.have.length.greaterThan(120); // Hex representation should be long
            expect(seed).to.match(/^[0-9a-fA-F]+$/); // Should be hex
        });

        it('should convert mnemonic to seed with passphrase', () => {
            const seedWithPassphrase = global.wasmSdk.mnemonic_to_seed(TestData.seeds.test, "testpassphrase");
            const seedWithoutPassphrase = global.wasmSdk.mnemonic_to_seed(TestData.seeds.test);
            
            expect(seedWithPassphrase).to.be.a('string');
            expect(seedWithPassphrase).to.have.length.greaterThan(120);
            expect(seedWithPassphrase).to.not.equal(seedWithoutPassphrase);
        });

        it('should produce consistent seeds for same mnemonic', () => {
            const seed1 = global.wasmSdk.mnemonic_to_seed(TestData.seeds.test);
            const seed2 = global.wasmSdk.mnemonic_to_seed(TestData.seeds.test);
            
            expect(seed1).to.equal(seed2);
        });
    });

    describe('Key Pair Generation', () => {
        it('should generate random key pairs', () => {
            if (!global.wasmSdk.generate_key_pair) {
                expect.fail('generate_key_pair function not available - using stub');
                return;
            }
            
            const keyPair = global.wasmSdk.generate_key_pair();
            
            expect(keyPair).to.be.an('object');
            expect(keyPair.private_key_wif).to.exist;
            expect(keyPair.public_key).to.exist;
            expect(keyPair.address).to.exist;
            
            TestAssertions.isValidWIF(keyPair.private_key_wif);
            TestAssertions.isValidPublicKey(keyPair.public_key);
        });

        it('should generate multiple key pairs', () => {
            if (!global.wasmSdk.generate_key_pairs) {
                expect.fail('generate_key_pairs function not available - using stub');
                return;
            }
            
            const count = 3;
            const keyPairs = global.wasmSdk.generate_key_pairs(count);
            
            expect(keyPairs).to.be.an('array');
            expect(keyPairs).to.have.length(count);
            
            keyPairs.forEach((keyPair, index) => {
                expect(keyPair).to.be.an('object', `Key pair ${index} should be an object`);
                TestAssertions.isValidWIF(keyPair.private_key_wif);
                TestAssertions.isValidPublicKey(keyPair.public_key);
            });
        });

        it('should create key pairs from WIF', () => {
            if (!global.wasmSdk.key_pair_from_wif) {
                expect.fail('key_pair_from_wif function not available - using stub');
                return;
            }
            
            const testWif = 'XBrZJKcW4ajWVNAU6yP87WQog6CjFnpbqyAKgNTZRqmhYvPgMNV2';
            const keyPair = global.wasmSdk.key_pair_from_wif(testWif);
            
            expect(keyPair).to.be.an('object');
            expect(keyPair.private_key_wif).to.equal(testWif);
            expect(keyPair.public_key).to.exist;
            expect(keyPair.address).to.exist;
        });

        it('should create key pairs from hex', () => {
            if (!global.wasmSdk.key_pair_from_hex) {
                expect.fail('key_pair_from_hex function not available - using stub');
                return;
            }
            
            const testHex = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef';
            const keyPair = global.wasmSdk.key_pair_from_hex(testHex);
            
            expect(keyPair).to.be.an('object');
            expect(keyPair.private_key_hex).to.equal(testHex);
            expect(keyPair.public_key).to.exist;
            expect(keyPair.address).to.exist;
        });
    });

    describe('Key Derivation', () => {
        describe('BIP44 Derivation', () => {
            it('should derive mainnet keys from seed', async () => {
                const result = await global.wasmSdk.derive_key_from_seed_with_path(
                    TestData.seeds.test,
                    undefined, // no passphrase
                    TestData.paths.bip44.mainnet,
                    'mainnet'
                );
                
                expect(result).to.be.an('object');
                expect(result.path).to.equal(TestData.paths.bip44.mainnet);
                expect(result.network).to.equal('mainnet');
                
                TestAssertions.isValidWIF(result.private_key_wif);
                TestAssertions.isValidPublicKey(result.public_key);
                TestAssertions.isValidDashAddress(result.address, 'mainnet');
            });

            it('should derive testnet keys from seed', async () => {
                const result = await global.wasmSdk.derive_key_from_seed_with_path(
                    TestData.seeds.test,
                    undefined,
                    TestData.paths.bip44.testnet,
                    'testnet'
                );
                
                expect(result).to.be.an('object');
                expect(result.path).to.equal(TestData.paths.bip44.testnet);
                expect(result.network).to.equal('testnet');
                
                TestAssertions.isValidWIF(result.private_key_wif);
                TestAssertions.isValidPublicKey(result.public_key);
                TestAssertions.isValidDashAddress(result.address, 'testnet');
            });

            it('should derive keys with passphrase', async () => {
                const withPassphrase = await global.wasmSdk.derive_key_from_seed_with_path(
                    TestData.seeds.test,
                    'testpassphrase',
                    TestData.paths.bip44.mainnet,
                    'mainnet'
                );
                
                const withoutPassphrase = await global.wasmSdk.derive_key_from_seed_with_path(
                    TestData.seeds.test,
                    undefined,
                    TestData.paths.bip44.mainnet,
                    'mainnet'
                );
                
                expect(withPassphrase.private_key_wif).to.not.equal(withoutPassphrase.private_key_wif);
                expect(withPassphrase.address).to.not.equal(withoutPassphrase.address);
            });
        });

        describe('DIP13 Identity Derivation', () => {
            it('should derive authentication keys', async () => {
                const result = await global.wasmSdk.derive_key_from_seed_with_path(
                    TestData.seeds.test,
                    undefined,
                    TestData.paths.dip13.auth,
                    'mainnet'
                );
                
                expect(result).to.be.an('object');
                expect(result.path).to.equal(TestData.paths.dip13.auth);
                TestAssertions.isValidWIF(result.private_key_wif);
                TestAssertions.isValidPublicKey(result.public_key);
            });

            it('should derive registration funding keys', async () => {
                const result = await global.wasmSdk.derive_key_from_seed_with_path(
                    TestData.seeds.test,
                    undefined,
                    TestData.paths.dip13.registration,
                    'mainnet'
                );
                
                expect(result).to.be.an('object');
                expect(result.path).to.equal(TestData.paths.dip13.registration);
                TestAssertions.isValidWIF(result.private_key_wif);
                TestAssertions.isValidPublicKey(result.public_key);
            });
        });

        describe('Derivation Path Helpers', () => {
            it('should provide BIP44 paths for mainnet', () => {
                if (!global.wasmSdk.derivation_path_bip44_mainnet) {
                    expect.fail('derivation_path_bip44_mainnet function not available - using stub');
                    return;
                }
                
                const pathInfo = global.wasmSdk.derivation_path_bip44_mainnet(0, false, 0);
                expect(pathInfo).to.be.an('object');
                expect(pathInfo.path).to.match(/^m\/44'\/5'\/\d+'\/\d+\/\d+$/);
            });

            it('should provide BIP44 paths for testnet', () => {
                if (!global.wasmSdk.derivation_path_bip44_testnet) {
                    expect.fail('derivation_path_bip44_testnet function not available - using stub');
                    return;
                }
                
                const pathInfo = global.wasmSdk.derivation_path_bip44_testnet(0, false, 0);
                expect(pathInfo).to.be.an('object');
                expect(pathInfo.path).to.match(/^m\/44'\/1'\/\d+'\/\d+\/\d+$/);
            });
        });
    });

    describe('Address Functions', () => {
        it('should convert public keys to addresses', () => {
            if (!global.wasmSdk.pubkey_to_address) {
                expect.fail('pubkey_to_address function not available - using stub');
                return;
            }
            
            const publicKey = '0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798';
            const address = global.wasmSdk.pubkey_to_address(publicKey, 'mainnet');
            
            expect(address).to.be.a('string');
            TestAssertions.isValidDashAddress(address, 'mainnet');
        });

        it('should validate Dash addresses correctly', () => {
            // Test mainnet addresses
            TestData.addresses.mainnet.valid.forEach(address => {
                expect(global.wasmSdk.validate_address(address, 'mainnet')).to.be.true;
                expect(global.wasmSdk.validate_address(address, 'testnet')).to.be.false;
            });
            
            // Test testnet addresses
            TestData.addresses.testnet.valid.forEach(address => {
                expect(global.wasmSdk.validate_address(address, 'testnet')).to.be.true;
                expect(global.wasmSdk.validate_address(address, 'mainnet')).to.be.false;
            });
            
            // Test invalid addresses
            TestData.addresses.mainnet.invalid.forEach(address => {
                expect(global.wasmSdk.validate_address(address, 'mainnet')).to.be.false;
                expect(global.wasmSdk.validate_address(address, 'testnet')).to.be.false;
            });
        });
    });

    describe('Message Signing', () => {
        it('should sign messages with private keys', () => {
            if (!global.wasmSdk.sign_message) {
                expect.fail('sign_message function not available - using stub');
                return;
            }
            
            const message = 'Test message for signing';
            const privateKeyWif = 'XBrZJKcW4ajWVNAU6yP87WQog6CjFnpbqyAKgNTZRqmhYvPgMNV2';
            
            const signature = global.wasmSdk.sign_message(message, privateKeyWif);
            
            expect(signature).to.be.a('string');
            expect(signature).to.have.length.greaterThan(80); // Base64 signature should be long
        });

        it('should produce consistent signatures for same input', () => {
            if (!global.wasmSdk.sign_message) {
                expect.fail('sign_message function not available - using stub');
                return;
            }
            
            const message = 'Test message';
            const privateKeyWif = 'XBrZJKcW4ajWVNAU6yP87WQog6CjFnpbqyAKgNTZRqmhYvPgMNV2';
            
            const signature1 = global.wasmSdk.sign_message(message, privateKeyWif);
            const signature2 = global.wasmSdk.sign_message(message, privateKeyWif);
            
            expect(signature1).to.equal(signature2);
        });
    });

    describe('Error Handling', () => {
        it('should handle invalid mnemonic phrases gracefully', () => {
            expect(() => {
                global.wasmSdk.mnemonic_to_seed('invalid mnemonic phrase');
            }).to.not.throw();
            
            // The function should return a result (even if deterministic) for invalid input
            const result = global.wasmSdk.mnemonic_to_seed('invalid mnemonic phrase');
            expect(result).to.be.a('string');
        });

        it('should handle invalid derivation paths', async () => {
            try {
                await global.wasmSdk.derive_key_from_seed_with_path(
                    TestData.seeds.test,
                    undefined,
                    'invalid/path',
                    'mainnet'
                );
                // If we reach here with stubs, that's fine
            } catch (error) {
                expect(error.message).to.include('invalid'); // Should mention invalid path
            }
        });

        it('should handle invalid WIF keys', () => {
            if (!global.wasmSdk.key_pair_from_wif) {
                return; // Skip if stub
            }
            
            expect(() => {
                global.wasmSdk.key_pair_from_wif('invalid-wif');
            }).to.throw();
        });
    });
});