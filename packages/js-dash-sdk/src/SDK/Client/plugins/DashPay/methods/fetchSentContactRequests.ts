/**
 *
 */
export async function fetchSentContactRequests(this: any, fromTimestamp = 0){
  const walletStore = this.storage.getWalletStore(this.walletId)
  const identities = walletStore.getIndexedIdentityIds();
  if(!identities.length){
    throw new Error('Require an identity to fetch sent contact requests');
  }
  const senderDashUniqueIdentityId = identities[0];
  const senderIdentity = await this.platform.identities.get(senderDashUniqueIdentityId);

  return this.platform.documents.get('dashpay.contactRequest', {
    where: [
      ['$ownerId', '==', senderIdentity.getId()],
      ['$createdAt', '>', fromTimestamp]
    ],
    orderBy:[
      ['$createdAt', 'asc']
    ]
  });
}
