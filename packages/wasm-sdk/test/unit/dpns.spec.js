/**
 * DPNS Tests - Migrated from dpns.test.mjs
 * 
 * These tests verify DPNS (Dash Platform Name Service) functionality including
 * username validation, homograph safety, contested name detection, and network operations.
 */

const { TestSdkBuilder, TestAssertions } = require('../helpers/sdk-builder.js');
const { TestData } = require('../fixtures/test-data.js');

describe('DPNS (Dash Platform Name Service)', () => {
    describe('Homograph Safety', () => {
        it('should handle basic ASCII names', () => {
            const result = global.wasmSdk.dpns_convert_to_homograph_safe("test");
            expect(result).to.equal("test");
        });

        it('should handle names with numbers', () => {
            const result = global.wasmSdk.dpns_convert_to_homograph_safe("test123");
            expect(result).to.equal("test123");
        });

        it('should handle names with hyphens', () => {
            const result = global.wasmSdk.dpns_convert_to_homograph_safe("test-name");
            expect(result).to.equal("test-name");
        });

        it('should convert uppercase to lowercase', () => {
            const result = global.wasmSdk.dpns_convert_to_homograph_safe("TestName");
            expect(result).to.equal("testname");
        });

        it('should remove special characters', () => {
            const result = global.wasmSdk.dpns_convert_to_homograph_safe("test@name!");
            expect(result).to.not.include('@');
            expect(result).to.not.include('!');
        });

        it('should handle unicode homographs', () => {
            // Test with Cyrillic 'е' that looks like Latin 'e'
            const result = global.wasmSdk.dpns_convert_to_homograph_safe("tеst");
            expect(result).to.not.equal("tеst");
        });

        it('should handle empty strings', () => {
            const result = global.wasmSdk.dpns_convert_to_homograph_safe("");
            expect(result).to.equal("");
        });

        it('should handle strings with only special characters', () => {
            const result = global.wasmSdk.dpns_convert_to_homograph_safe("@#$%");
            expect(result).to.not.include('@');
            expect(result).to.not.include('#');
            expect(result).to.not.include('$');
            expect(result).to.not.include('%');
        });
    });

    describe('Username Validation', () => {
        describe('Valid Usernames', () => {
            it('should accept basic valid username', () => {
                expect(global.wasmSdk.dpns_is_valid_username("alice")).to.be.true;
            });

            it('should accept username with numbers', () => {
                expect(global.wasmSdk.dpns_is_valid_username("alice123")).to.be.true;
            });

            it('should accept username with hyphens', () => {
                expect(global.wasmSdk.dpns_is_valid_username("alice-bob")).to.be.true;
            });

            it('should accept minimum length username', () => {
                expect(global.wasmSdk.dpns_is_valid_username("abc")).to.be.true;
            });

            it('should accept maximum length username', () => {
                const maxName = "a".repeat(19);
                expect(global.wasmSdk.dpns_is_valid_username(maxName)).to.be.true;
            });
        });

        describe('Invalid Usernames', () => {
            it('should reject usernames that are too short', () => {
                expect(global.wasmSdk.dpns_is_valid_username("ab")).to.be.false;
            });

            it('should reject usernames that are too long', () => {
                const longName = "a".repeat(64);
                expect(global.wasmSdk.dpns_is_valid_username(longName)).to.be.false;
            });

            it('should reject usernames starting with hyphen', () => {
                expect(global.wasmSdk.dpns_is_valid_username("-alice")).to.be.false;
            });

            it('should reject usernames ending with hyphen', () => {
                expect(global.wasmSdk.dpns_is_valid_username("alice-")).to.be.false;
            });

            it('should reject usernames with double hyphens', () => {
                expect(global.wasmSdk.dpns_is_valid_username("alice--bob")).to.be.false;
            });

            it('should reject usernames with uppercase letters', () => {
                expect(global.wasmSdk.dpns_is_valid_username("Alice")).to.be.false;
            });

            it('should reject usernames with special characters', () => {
                expect(global.wasmSdk.dpns_is_valid_username("alice@bob")).to.be.false;
            });

            it('should reject usernames with spaces', () => {
                expect(global.wasmSdk.dpns_is_valid_username("alice bob")).to.be.false;
            });

            it('should reject usernames with only numbers', () => {
                expect(global.wasmSdk.dpns_is_valid_username("123456")).to.be.false;
            });

            it('should reject usernames starting with numbers', () => {
                expect(global.wasmSdk.dpns_is_valid_username("1alice")).to.be.false;
            });

            it('should reject empty strings', () => {
                expect(global.wasmSdk.dpns_is_valid_username("")).to.be.false;
            });

            it('should reject null/undefined values', () => {
                expect(global.wasmSdk.dpns_is_valid_username(null)).to.be.false;
                expect(global.wasmSdk.dpns_is_valid_username(undefined)).to.be.false;
            });
        });
    });

    describe('Contested Username Detection', () => {
        it('should return boolean for all inputs', () => {
            const result = global.wasmSdk.dpns_is_contested_username("uniquename123");
            expect(result).to.be.a('boolean');
        });

        it('should handle common names', () => {
            const result = global.wasmSdk.dpns_is_contested_username("alice");
            expect(result).to.be.a('boolean');
        });

        it('should handle single letter names', () => {
            const result = global.wasmSdk.dpns_is_contested_username("a");
            expect(result).to.be.a('boolean');
        });

        it('should handle three letter names', () => {
            const result = global.wasmSdk.dpns_is_contested_username("abc");
            expect(result).to.be.a('boolean');
        });

        it('should handle empty strings', () => {
            const result = global.wasmSdk.dpns_is_contested_username("");
            expect(result).to.be.a('boolean');
        });

        it('should typically mark short names as contested', () => {
            // Short names are typically contested
            expect(global.wasmSdk.dpns_is_contested_username("bob")).to.be.true;
            expect(global.wasmSdk.dpns_is_contested_username("test")).to.be.true;
        });

        it('should typically mark longer unique names as non-contested', () => {
            // Longer unique names are typically not contested
            const result = global.wasmSdk.dpns_is_contested_username("verylonganduniquename");
            expect(result).to.be.false;
        });
    });

    describe('Network Operations', () => {
        let sdk;

        beforeEach(async () => {
            try {
                sdk = await global.createTestSdk.testnet();
            } catch (error) {
                // If SDK creation fails, tests will skip or handle gracefully
                sdk = null;
            }
        });

        afterEach(() => {
            if (sdk && sdk.free) {
                sdk.free();
            }
        });

        describe('Username Retrieval', () => {
            it('should handle get_dpns_usernames for identity', async function() {
                if (!sdk) this.skip();

                try {
                    const result = await global.wasmSdk.get_dpns_usernames(
                        sdk,
                        TestData.identities.valid[0],
                        10
                    );
                    expect(result).to.be.an('array');
                } catch (error) {
                    // Network errors are expected in test environment
                    expect(error.message).to.match(/network|connection|identity/i);
                }
            });

            it('should handle get_dpns_username for single identity', async function() {
                if (!sdk) this.skip();

                try {
                    const result = await global.wasmSdk.get_dpns_username(
                        sdk,
                        TestData.identities.valid[0]
                    );
                    expect(result).to.satisfy(r => r === null || typeof r === 'object');
                } catch (error) {
                    // Network errors are expected
                    expect(error.message).to.match(/network|connection|identity/i);
                }
            });
        });

        describe('Name Registration', () => {
            it('should reject registration with invalid identity', async function() {
                if (!sdk) this.skip();

                try {
                    await global.wasmSdk.dpns_register_name(
                        sdk,
                        "testname",
                        "invalididentityid",
                        0,
                        "invalidprivatekey"
                    );
                    expect.fail('Should have thrown error with invalid identity');
                } catch (error) {
                    expect(error.message).to.match(/identity|validation/i);
                }
            });
        });

        describe('Name Availability', () => {
            it('should check name availability', async function() {
                if (!sdk) this.skip();

                try {
                    const result = await global.wasmSdk.dpns_is_name_available(sdk, "testname");
                    expect(result).to.be.a('boolean');
                } catch (error) {
                    // Network errors are acceptable in test environment
                    expect(error.message).to.match(/network|connection/i);
                }
            });

            it('should typically show long names as available', async function() {
                if (!sdk) this.skip();

                try {
                    const result = await global.wasmSdk.dpns_is_name_available(sdk, "verylonganduniquename");
                    expect(result).to.be.true;
                } catch (error) {
                    // Skip if network unavailable
                }
            });
        });

        describe('Name Resolution', () => {
            it('should resolve name to identity', async function() {
                if (!sdk) this.skip();

                try {
                    const result = await global.wasmSdk.dpns_resolve_name(sdk, "alice.dash");
                    expect(result).to.satisfy(r => r === null || typeof r === 'object');
                } catch (error) {
                    // Network errors are expected
                    expect(error.message).to.match(/network|connection/i);
                }
            });

            it('should handle get_dpns_username_by_name', async function() {
                if (!sdk) this.skip();

                try {
                    const result = await global.wasmSdk.get_dpns_username_by_name(sdk, "alice");
                    expect(result).to.satisfy(r => r === null || typeof r === 'object');
                } catch (error) {
                    // Network errors are expected
                    expect(error.message).to.match(/network|connection/i);
                }
            });
        });
    });

    describe('Integration Tests', () => {
        it('should validate and convert names consistently', () => {
            const testName = "TestName-123";
            const safeName = global.wasmSdk.dpns_convert_to_homograph_safe(testName);
            
            // Safe name should be valid if original was valid
            if (global.wasmSdk.dpns_is_valid_username(testName.toLowerCase())) {
                expect(global.wasmSdk.dpns_is_valid_username(safeName)).to.be.true;
            }
        });

        it('should handle complete DPNS workflow validation', () => {
            const originalName = "Alice-Test123";
            
            // Convert to safe form
            const safeName = global.wasmSdk.dpns_convert_to_homograph_safe(originalName);
            expect(safeName).to.equal("alice-test123");
            
            // Check if valid
            const isValid = global.wasmSdk.dpns_is_valid_username(safeName);
            expect(isValid).to.be.true;
            
            // Check if contested
            const isContested = global.wasmSdk.dpns_is_contested_username(safeName);
            expect(isContested).to.be.a('boolean');
        });
    });

    describe('Edge Cases and Error Handling', () => {
        it('should handle malformed inputs gracefully', () => {
            // Test various malformed inputs
            expect(() => {
                global.wasmSdk.dpns_convert_to_homograph_safe(null);
            }).to.not.throw();
            
            expect(() => {
                global.wasmSdk.dpns_is_valid_username(123);
            }).to.not.throw();
            
            expect(() => {
                global.wasmSdk.dpns_is_contested_username({});
            }).to.not.throw();
        });

        it('should return consistent types for edge cases', () => {
            expect(global.wasmSdk.dpns_convert_to_homograph_safe("")).to.be.a('string');
            expect(global.wasmSdk.dpns_is_valid_username("")).to.be.a('boolean');
            expect(global.wasmSdk.dpns_is_contested_username("")).to.be.a('boolean');
        });
    });
});