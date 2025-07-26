// key-derivation.test.js - Tests for key derivation functions
const { loadWasmModule } = require('./test-setup');

describe('Key Derivation Tests', () => {
    let wasm;
    const testSeed = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    
    beforeAll(async () => {
        wasm = await loadWasmModule();
    });
    
    describe('derive_key_from_seed_with_path', () => {
        test('should derive BIP44 mainnet key', async () => {
            const path = "m/44'/5'/0'/0/0";
            const result = await wasm.derive_key_from_seed_with_path(
                testSeed,
                undefined,
                path,
                "mainnet"
            );
            
            console.log('BIP44 result:', result);
            
            expect(result).toBeDefined();
            expect(result.path).toBe(path);
            expect(result.private_key_wif).toBeDefined();
            expect(result.private_key_hex).toBeDefined();
            expect(result.public_key).toBeDefined();
            expect(result.address).toBeDefined();
            expect(result.network).toBe("mainnet");
        });
        
        test('should derive DIP13 authentication key', async () => {
            const path = "m/9'/5'/5'/0'/0'/0'/0'";
            const result = await wasm.derive_key_from_seed_with_path(
                testSeed,
                undefined,
                path,
                "mainnet"
            );
            
            console.log('DIP13 Authentication key result:', result);
            
            expect(result).toBeDefined();
            expect(result.path).toBe(path);
            expect(result.private_key_wif).toBeDefined();
            expect(result.address).toBeDefined();
        });
        
        test('should derive DIP13 registration funding key', async () => {
            const path = "m/9'/5'/5'/1'/0";
            const result = await wasm.derive_key_from_seed_with_path(
                testSeed,
                undefined,
                path,
                "mainnet"
            );
            
            console.log('DIP13 Registration funding key result:', result);
            
            expect(result).toBeDefined();
            expect(result.path).toBe(path);
            expect(result.private_key_wif).toBeDefined();
        });
        
        test('should work with passphrase', async () => {
            const path = "m/44'/5'/0'/0/0";
            const passphrase = "test passphrase";
            const result = await wasm.derive_key_from_seed_with_path(
                testSeed,
                passphrase,
                path,
                "mainnet"
            );
            
            console.log('With passphrase result:', result);
            
            expect(result).toBeDefined();
            expect(result.path).toBe(path);
            // Address should be different with passphrase
            const withoutPassphrase = await wasm.derive_key_from_seed_with_path(
                testSeed,
                undefined,
                path,
                "mainnet"
            );
            expect(result.address).not.toBe(withoutPassphrase.address);
        });
        
        test('should work on testnet', async () => {
            const path = "m/44'/1'/0'/0/0";
            const result = await wasm.derive_key_from_seed_with_path(
                testSeed,
                undefined,
                path,
                "testnet"
            );
            
            console.log('Testnet result:', result);
            
            expect(result).toBeDefined();
            expect(result.network).toBe("testnet");
            expect(result.address).toMatch(/^y/); // Testnet addresses start with 'y'
        });
    });
    
    describe('generate_mnemonic', () => {
        test('should generate 12-word mnemonic', async () => {
            const mnemonic = await wasm.generate_mnemonic(12);
            const words = mnemonic.split(' ');
            
            expect(words.length).toBe(12);
            expect(wasm.validate_mnemonic(mnemonic)).toBe(true);
        });
        
        test('should generate 24-word mnemonic', async () => {
            const mnemonic = await wasm.generate_mnemonic(24);
            const words = mnemonic.split(' ');
            
            expect(words.length).toBe(24);
            expect(wasm.validate_mnemonic(mnemonic)).toBe(true);
        });
        
        test('should generate mnemonic in different languages', async () => {
            const languages = ['en', 'es', 'fr', 'it', 'ja', 'ko', 'pt', 'cs'];
            
            for (const lang of languages) {
                const mnemonic = await wasm.generate_mnemonic(12, lang);
                console.log(`${lang} mnemonic:`, mnemonic.substring(0, 30) + '...');
                expect(wasm.validate_mnemonic(mnemonic, lang)).toBe(true);
            }
        });
    });
    
    describe('DIP13 paths', () => {
        test('should create correct DIP13 mainnet path info', () => {
            const result = wasm.derivation_path_dip13_mainnet(0);
            
            expect(result).toBeDefined();
            expect(result.path).toBe("m/9'/5'/0'");
            expect(result.purpose).toBe(9);
            expect(result.coin_type).toBe(5);
            expect(result.account).toBe(0);
        });
        
        test('should create correct DIP13 testnet path info', () => {
            const result = wasm.derivation_path_dip13_testnet(0);
            
            expect(result).toBeDefined();
            expect(result.path).toBe("m/9'/1'/0'");
            expect(result.purpose).toBe(9);
            expect(result.coin_type).toBe(1);
            expect(result.account).toBe(0);
        });
    });
});