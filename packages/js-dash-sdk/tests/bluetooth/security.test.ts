import { BluetoothSecurity } from '../../src/bluetooth/security/BluetoothSecurity';

describe('BluetoothSecurity', () => {
  let security: BluetoothSecurity;

  beforeEach(() => {
    security = new BluetoothSecurity();
  });

  describe('key generation', () => {
    it('should generate ECDH key pair', async () => {
      const { publicKey, privateKey } = await security.generateKeyPair();

      expect(publicKey).toBeInstanceOf(Uint8Array);
      expect(publicKey.length).toBe(65); // Uncompressed P-256 public key
      expect(privateKey).toBeDefined();
      expect(privateKey.privateKey).toBeDefined();
      expect(privateKey.publicKey).toBeDefined();
    });

    it('should generate different keys each time', async () => {
      const key1 = await security.generateKeyPair();
      const key2 = await security.generateKeyPair();

      expect(key1.publicKey).not.toEqual(key2.publicKey);
    });
  });

  describe('pairing codes', () => {
    it('should generate 9-digit pairing codes', () => {
      const code = BluetoothSecurity.generatePairingCode();
      
      expect(code).toMatch(/^\d{3}-\d{3}-\d{3}$/);
      expect(code.replace(/-/g, '').length).toBe(9);
    });

    it('should generate unique pairing codes', () => {
      const codes = new Set(
        Array.from({ length: 100 }, () => BluetoothSecurity.generatePairingCode())
      );

      expect(codes.size).toBeGreaterThan(90); // Allow for some duplicates due to randomness
    });

    it('should verify pairing codes correctly', () => {
      const code = '123-456-789';
      
      expect(BluetoothSecurity.verifyPairingCode(code, code)).toBe(true);
      expect(BluetoothSecurity.verifyPairingCode(code, '123-456-788')).toBe(false);
      expect(BluetoothSecurity.verifyPairingCode(code, '123-456')).toBe(false);
    });

    it('should use constant-time comparison', () => {
      const code1 = '000-000-000';
      const code2 = '999-999-999';
      
      // Measure multiple comparisons
      const timings: number[] = [];
      
      for (let i = 0; i < 1000; i++) {
        const start = performance.now();
        BluetoothSecurity.verifyPairingCode(code1, code2);
        const end = performance.now();
        timings.push(end - start);
      }
      
      // Check that timing variations are minimal
      const avgTime = timings.reduce((a, b) => a + b) / timings.length;
      const variance = timings.reduce((sum, t) => sum + Math.pow(t - avgTime, 2), 0) / timings.length;
      
      expect(variance).toBeLessThan(0.01); // Low variance indicates constant-time
    });
  });

  describe('challenges', () => {
    it('should generate 32-byte challenges', () => {
      const challenge = BluetoothSecurity.generateChallenge();
      
      expect(challenge).toBeInstanceOf(Uint8Array);
      expect(challenge.length).toBe(32);
    });

    it('should generate unique challenges', () => {
      const challenges = Array.from({ length: 10 }, () => 
        BluetoothSecurity.generateChallenge()
      );
      
      const uniqueChallenges = new Set(challenges.map(c => 
        Array.from(c).join(',')
      ));
      
      expect(uniqueChallenges.size).toBe(10);
    });
  });

  describe('session management', () => {
    it('should track session state', () => {
      expect(security.hasSession()).toBe(false);
      
      // After key exchange would be performed
      // expect(security.hasSession()).toBe(true);
    });

    it('should clear session properly', () => {
      security.clearSession();
      expect(security.hasSession()).toBe(false);
    });
  });

  describe('encryption/decryption', () => {
    it('should handle encryption when no session exists', async () => {
      const data = new TextEncoder().encode('test data');
      
      await expect(security.encrypt(data))
        .rejects.toThrow('Session key not established');
    });

    it('should handle decryption when no session exists', async () => {
      const encrypted = new Uint8Array(32);
      const iv = new Uint8Array(12);
      const tag = new Uint8Array(16);
      
      await expect(security.decrypt(encrypted, iv, tag))
        .rejects.toThrow('Session key not established');
    });
  });

  describe('challenge-response authentication', () => {
    let keyPair: CryptoKeyPair;

    beforeEach(async () => {
      // Generate test key pair
      keyPair = await crypto.subtle.generateKey(
        {
          name: 'ECDSA',
          namedCurve: 'P-256'
        },
        true,
        ['sign', 'verify']
      );
    });

    it('should create challenge response', async () => {
      const challenge = BluetoothSecurity.generateChallenge();
      
      const { response, timestamp } = await security.createChallengeResponse(
        challenge,
        keyPair.privateKey
      );
      
      expect(response).toBeInstanceOf(Uint8Array);
      expect(response.length).toBeGreaterThan(0);
      expect(timestamp).toBeCloseTo(Date.now(), -2);
    });

    it('should verify valid challenge response', async () => {
      const challenge = BluetoothSecurity.generateChallenge();
      
      // Create response
      const { response, timestamp } = await security.createChallengeResponse(
        challenge,
        keyPair.privateKey
      );
      
      // Export public key
      const publicKeyData = await crypto.subtle.exportKey('raw', keyPair.publicKey);
      
      // Verify
      const isValid = await security.verifyChallengeResponse(
        challenge,
        response,
        timestamp,
        new Uint8Array(publicKeyData),
        60000
      );
      
      expect(isValid).toBe(true);
    });

    it('should reject expired challenge response', async () => {
      const challenge = BluetoothSecurity.generateChallenge();
      
      const { response, timestamp } = await security.createChallengeResponse(
        challenge,
        keyPair.privateKey
      );
      
      const publicKeyData = await crypto.subtle.exportKey('raw', keyPair.publicKey);
      
      // Verify with very old timestamp
      const isValid = await security.verifyChallengeResponse(
        challenge,
        response,
        timestamp - 120000, // 2 minutes ago
        new Uint8Array(publicKeyData),
        60000 // 1 minute max age
      );
      
      expect(isValid).toBe(false);
    });

    it('should reject tampered challenge response', async () => {
      const challenge = BluetoothSecurity.generateChallenge();
      
      const { response, timestamp } = await security.createChallengeResponse(
        challenge,
        keyPair.privateKey
      );
      
      // Tamper with response
      response[0] = response[0] ^ 0xFF;
      
      const publicKeyData = await crypto.subtle.exportKey('raw', keyPair.publicKey);
      
      const isValid = await security.verifyChallengeResponse(
        challenge,
        response,
        timestamp,
        new Uint8Array(publicKeyData),
        60000
      );
      
      expect(isValid).toBe(false);
    });
  });
});