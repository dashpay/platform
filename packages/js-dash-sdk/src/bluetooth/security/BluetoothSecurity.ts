/**
 * Security layer for Bluetooth communication
 * Implements encryption and authentication
 */

export class BluetoothSecurity {
  private sharedSecret: CryptoKey | null = null;
  private sessionKey: CryptoKey | null = null;
  private nonce: number = 0;

  /**
   * Generate ECDH key pair for key exchange
   */
  async generateKeyPair(): Promise<{
    publicKey: Uint8Array;
    privateKey: CryptoKeyPair;
  }> {
    const keyPair = await crypto.subtle.generateKey(
      {
        name: 'ECDH',
        namedCurve: 'P-256'
      },
      true,
      ['deriveBits']
    );

    const publicKeyData = await crypto.subtle.exportKey('raw', keyPair.publicKey);
    
    return {
      publicKey: new Uint8Array(publicKeyData),
      privateKey: keyPair
    };
  }

  /**
   * Perform ECDH key exchange
   */
  async performKeyExchange(
    privateKey: CryptoKey,
    remotePublicKey: Uint8Array
  ): Promise<void> {
    // Import remote public key
    const remoteKey = await crypto.subtle.importKey(
      'raw',
      remotePublicKey,
      {
        name: 'ECDH',
        namedCurve: 'P-256'
      },
      false,
      []
    );

    // Derive shared secret
    const sharedBits = await crypto.subtle.deriveBits(
      {
        name: 'ECDH',
        public: remoteKey
      },
      privateKey,
      256
    );

    // Import as AES key
    this.sharedSecret = await crypto.subtle.importKey(
      'raw',
      sharedBits,
      { name: 'AES-GCM' },
      false,
      ['encrypt', 'decrypt']
    );

    // Derive session key
    await this.deriveSessionKey();
  }

  /**
   * Derive session key from shared secret
   */
  private async deriveSessionKey(): Promise<void> {
    if (!this.sharedSecret) {
      throw new Error('Shared secret not established');
    }

    // Use HKDF to derive session key
    const salt = crypto.getRandomValues(new Uint8Array(16));
    const info = new TextEncoder().encode('dash-bluetooth-session');
    
    // Export shared secret for HKDF
    const sharedSecretData = await crypto.subtle.exportKey('raw', this.sharedSecret);
    
    // Import as HKDF key
    const hkdfKey = await crypto.subtle.importKey(
      'raw',
      sharedSecretData,
      { name: 'HKDF' },
      false,
      ['deriveBits']
    );
    
    // Derive session key bits
    const sessionKeyBits = await crypto.subtle.deriveBits(
      {
        name: 'HKDF',
        salt,
        info,
        hash: 'SHA-256'
      },
      hkdfKey,
      256
    );
    
    // Import as AES key
    this.sessionKey = await crypto.subtle.importKey(
      'raw',
      sessionKeyBits,
      { name: 'AES-GCM' },
      false,
      ['encrypt', 'decrypt']
    );
  }

  /**
   * Encrypt data
   */
  async encrypt(data: Uint8Array): Promise<{
    encrypted: Uint8Array;
    iv: Uint8Array;
    tag: Uint8Array;
  }> {
    if (!this.sessionKey) {
      throw new Error('Session key not established');
    }

    // Generate IV with counter
    const iv = new Uint8Array(12);
    crypto.getRandomValues(iv);
    
    // Include nonce to prevent replay attacks
    const nonceBytes = new Uint8Array(8);
    new DataView(nonceBytes.buffer).setBigUint64(0, BigInt(this.nonce++));
    iv.set(nonceBytes, 4);

    // Encrypt
    const encrypted = await crypto.subtle.encrypt(
      {
        name: 'AES-GCM',
        iv,
        tagLength: 128
      },
      this.sessionKey,
      data
    );

    // Extract tag (last 16 bytes)
    const encryptedArray = new Uint8Array(encrypted);
    const ciphertext = encryptedArray.slice(0, -16);
    const tag = encryptedArray.slice(-16);

    return {
      encrypted: ciphertext,
      iv,
      tag
    };
  }

  /**
   * Decrypt data
   */
  async decrypt(
    encrypted: Uint8Array,
    iv: Uint8Array,
    tag: Uint8Array
  ): Promise<Uint8Array> {
    if (!this.sessionKey) {
      throw new Error('Session key not established');
    }

    // Combine ciphertext and tag
    const combined = new Uint8Array(encrypted.length + tag.length);
    combined.set(encrypted);
    combined.set(tag, encrypted.length);

    // Decrypt
    const decrypted = await crypto.subtle.decrypt(
      {
        name: 'AES-GCM',
        iv,
        tagLength: 128
      },
      this.sessionKey,
      combined
    );

    return new Uint8Array(decrypted);
  }

  /**
   * Sign data for authentication
   */
  async sign(data: Uint8Array, privateKey: CryptoKey): Promise<Uint8Array> {
    const signature = await crypto.subtle.sign(
      {
        name: 'ECDSA',
        hash: 'SHA-256'
      },
      privateKey,
      data
    );

    return new Uint8Array(signature);
  }

  /**
   * Verify signature
   */
  async verify(
    data: Uint8Array,
    signature: Uint8Array,
    publicKey: Uint8Array
  ): Promise<boolean> {
    // Import public key
    const key = await crypto.subtle.importKey(
      'raw',
      publicKey,
      {
        name: 'ECDSA',
        namedCurve: 'P-256'
      },
      false,
      ['verify']
    );

    return crypto.subtle.verify(
      {
        name: 'ECDSA',
        hash: 'SHA-256'
      },
      key,
      signature,
      data
    );
  }

  /**
   * Generate secure pairing code
   */
  static generatePairingCode(): string {
    const code = crypto.getRandomValues(new Uint8Array(3));
    return Array.from(code)
      .map(b => b.toString(10).padStart(3, '0'))
      .join('-');
  }

  /**
   * Verify pairing code
   */
  static verifyPairingCode(code: string, expected: string): boolean {
    // Constant-time comparison to prevent timing attacks
    if (code.length !== expected.length) return false;
    
    let diff = 0;
    for (let i = 0; i < code.length; i++) {
      diff |= code.charCodeAt(i) ^ expected.charCodeAt(i);
    }
    
    return diff === 0;
  }

  /**
   * Clear session keys
   */
  clearSession(): void {
    this.sharedSecret = null;
    this.sessionKey = null;
    this.nonce = 0;
  }

  /**
   * Check if session is established
   */
  hasSession(): boolean {
    return this.sessionKey !== null;
  }

  /**
   * Generate challenge for authentication
   */
  static generateChallenge(): Uint8Array {
    return crypto.getRandomValues(new Uint8Array(32));
  }

  /**
   * Create response to authentication challenge
   */
  async createChallengeResponse(
    challenge: Uint8Array,
    privateKey: CryptoKey
  ): Promise<{
    response: Uint8Array;
    timestamp: number;
  }> {
    const timestamp = Date.now();
    
    // Combine challenge and timestamp
    const data = new Uint8Array(challenge.length + 8);
    data.set(challenge);
    new DataView(data.buffer).setBigUint64(challenge.length, BigInt(timestamp));
    
    // Sign the combined data
    const response = await this.sign(data, privateKey);
    
    return {
      response,
      timestamp
    };
  }

  /**
   * Verify challenge response
   */
  async verifyChallengeResponse(
    challenge: Uint8Array,
    response: Uint8Array,
    timestamp: number,
    publicKey: Uint8Array,
    maxAge: number = 30000 // 30 seconds
  ): Promise<boolean> {
    // Check timestamp
    const age = Date.now() - timestamp;
    if (age > maxAge || age < 0) {
      return false;
    }
    
    // Recreate the signed data
    const data = new Uint8Array(challenge.length + 8);
    data.set(challenge);
    new DataView(data.buffer).setBigUint64(challenge.length, BigInt(timestamp));
    
    // Verify signature
    return this.verify(data, response, publicKey);
  }
}