/**
 *
 */
export async function fetchReceivedContactRequests(this: any, fromTimestamp = 0){
  const walletStore = this.storage.getWalletStore(this.walletId)
  const identities = walletStore.getIndexedIdentityIds();
  if(!identities.length){
    throw new Error('Require an identity to fetch sent contact requests');
  }
  const receiverDashUniqueIdentityId = identities[0];
  const receiverIdentity = await this.platform.identities.get(receiverDashUniqueIdentityId);

  return this.platform.documents.get('dashpay.contactRequest', {
    where: [
      ['toUserId', '==', receiverIdentity.getId()],
      ['$createdAt', '>', fromTimestamp]
    ],
  });
}
