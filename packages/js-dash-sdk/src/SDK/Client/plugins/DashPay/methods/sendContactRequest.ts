
/**
 *
 * @param contactName
 * @param accountLabel
 */
export async function sendContactRequest(this: any, contactName, accountLabel = 'Default account'){
  // @ts-ignore
  const walletStore = this.storage.getWalletStore(this.walletId);
  // @ts-ignore
  const identities = walletStore.getIndexedIdentityIds();
  if(!identities.length){
    throw new Error('Require an identity to send a contact request');
  }
  const senderDashUniqueIdentityId = identities[0];
  const senderIdentity = await this.platform.identities.get(senderDashUniqueIdentityId);

  const senderHdPrivateKey = this.identities.getIdentityHDKeyByIndex(0, 0);

  const senderPrivateKey = senderHdPrivateKey.privateKey;
  const senderPrivateKeyBuffer = senderPrivateKey.toBuffer();
  const retrieveContactName = await this.platform.names.resolve(`${contactName}.dash`);
  if(!retrieveContactName) throw new Error(`No such name found for ${contactName}.dash`);

  const contactDashUniqueIdentityId = retrieveContactName.ownerId.toString();
  const receiverIdentity = await this.platform.identities.get(retrieveContactName.ownerId);
  const receiverPublicKey = receiverIdentity.toJSON().publicKeys[0].data;
  const receiverPublicKeyBuffer = Buffer.from(receiverPublicKey, 'base64');

  const extendedPrivateKey = this.keyChainStore.getMasterKeyChain().getDIP15ExtendedKey('0x'+ senderDashUniqueIdentityId, '0x'+contactDashUniqueIdentityId);
  const extendedPublicKey = extendedPrivateKey.hdPublicKey;

  const extendedPublicKeyBuffers = Buffer.concat([extendedPublicKey._buffers.parentFingerPrint, extendedPublicKey._buffers.chainCode, extendedPublicKey._buffers.publicKey]);

  const sharedSecret = this.encryptSharedKey(senderPrivateKeyBuffer, receiverPublicKeyBuffer);
  const accountReference = this.createAccountReference(senderPrivateKeyBuffer, extendedPublicKey.toBuffer());
  const encryptedPublicKey = this.encryptPublicKey(extendedPublicKeyBuffers, sharedSecret);
  const encryptedPublicKeyBuffer = Buffer.from(encryptedPublicKey, 'hex');
  const encryptedAccountLabelBuffer = Buffer.from(this.encryptAccountLabel(sharedSecret, accountLabel), 'base64')
  const contactRequest = {
    toUserId: receiverIdentity.getId(),
    encryptedPublicKey: encryptedPublicKeyBuffer ,
    senderKeyIndex: 0,
    recipientKeyIndex: 0,
    accountReference,
    encryptedAccountLabel: encryptedAccountLabelBuffer ,
  };
  const contactRequestDocument = await this.platform.documents.create(
    'dashpay.contactRequest',
    senderIdentity,
    contactRequest,
  );

  const documentBatch = {
    create: [contactRequestDocument],
    replace: [],
    delete: [],
  };

  return this.platform.documents.broadcast(documentBatch, senderIdentity);
}
