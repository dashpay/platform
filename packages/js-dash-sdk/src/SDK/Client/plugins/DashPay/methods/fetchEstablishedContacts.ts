import { Contact } from "../types/Contact";
import { HDPublicKey } from "@dashevo/dashcore-lib";
import {KeyChain} from "@dashevo/wallet-lib";

/**
 * Fetch establish contact from a specific timestamp
 * @param fromTimestamp {Number} - optional timestamp
 * @return {Contact[]} establishedContact - All established contacts
 */
export async function fetchEstablishedContacts(this: any, fromTimestamp = 0) {
  const establishedContacts: Contact[] = [];
  const receivedContactRequests = await this.fetchReceivedContactRequests(fromTimestamp);
  const receivedContactRequestsSenderIds = receivedContactRequests.map((sentContactRequest) => sentContactRequest.ownerId.toString());

  const sentContactRequests = await this.fetchSentContactRequests(fromTimestamp);
  const sentContactRequestsReceiverIds = sentContactRequests.map((sentContactRequest) => sentContactRequest.data.toUserId.toString());

  const infoFetchingPromises: Promise<any>[] = [];

  sentContactRequestsReceiverIds.forEach((sentContactRequestsReceiverId, sentIdsArrayIndex) => {
    const receivedIdsArrayIndex = receivedContactRequestsSenderIds.indexOf(sentContactRequestsReceiverId);
    if (receivedIdsArrayIndex !== -1) {
      const contact = new Contact(sentContactRequestsReceiverId, sentContactRequests[sentIdsArrayIndex], receivedContactRequests[receivedIdsArrayIndex]);

      const fetchAndPopulateDPNSRecord = () => {
        return this.platform.names
          .resolveByRecord('dashUniqueIdentityId', contact.identityId)
          .then((record) => contact.setUsername(record && record[0] && record[0].data.label));
      }

      const fetchAndPopulateIdentity = () => {
        return this.platform.identities
          .get(contact.identityId)
          .then((identity) => contact.setIdentity(identity));
      }

      const fetchAndPopulateDashPayProfile = () => {
        return this.fetchProfile(contact.identityId)
          .then((profile) => contact.setProfile(profile));
      }

      // We need to fetch DPNS record, DashPay public data and identity for each contact
      const infoFetchingPromise = fetchAndPopulateDPNSRecord()
        .then(fetchAndPopulateIdentity)
        .then(fetchAndPopulateDashPayProfile);

      infoFetchingPromises.push(infoFetchingPromise);
      establishedContacts.push(contact);


    }
  });
  await Promise.all(infoFetchingPromises);
  const walletStore = this.storage.getWalletStore(this.walletId)
  const selfIdentitiesIds = walletStore.getIndexedIdentityIds();
  const selfIdentitiesPrimises = [];
  const selfIdentities = [];
  selfIdentitiesIds.forEach((selfIdentitiesId) => {
    // @ts-ignore
    selfIdentitiesPrimises.push(this.platform.identities
      .get(selfIdentitiesId)
      // @ts-ignore
      .then((id) => selfIdentities.push(id)
      ));
  });
  await Promise.all(selfIdentitiesPrimises);

  establishedContacts
    .forEach((establishedContact) => {
      const senderKey = establishedContact.identity.publicKeys[establishedContact.receivedRequest.data.senderKeyIndex];
      // const receiverKey = selfIdentities[0].publicKeys[establishedContact.receivedRequest.data.recipientKeyIndex];

      // @ts-ignore
      const {privateKey} = this.identities.getIdentityHDKeyById(selfIdentities[0].getId().toString(), 0);

      const sharedSecret = this.encryptSharedKey(privateKey.toBuffer(), senderKey.data);

      const receivedRequestEncryptedPublicKeyBuffer = establishedContact.receivedRequest.data.encryptedPublicKey;
      const receivedDecryptedPublicKeyBuffer = Buffer.from(this.decryptPublicKey(receivedRequestEncryptedPublicKeyBuffer, sharedSecret), 'hex');

      const receivedContactPublicKey = HDPublicKey.fromObject({
        parentFingerPrint: receivedDecryptedPublicKeyBuffer.slice(0, 4),
        chainCode: receivedDecryptedPublicKeyBuffer.slice(4, 36),
        publicKey: receivedDecryptedPublicKeyBuffer.slice(36, 69),
        network: 'testnet',
        depth: Buffer.from('07', 'hex'),
        childIndex: Buffer.alloc(4),
      })

      // @ts-ignore
      const extendedPrivateKey = this.keyChainStore.getMasterKeyChain().getDIP15ExtendedKey('0x'+ `${selfIdentities[0].getId().toString()}`, '0x'+`${establishedContact.identity.getId().toString()}`);

      establishedContact.setHDKeys({
        sending: receivedContactPublicKey,
        receiving: extendedPrivateKey
      });

      const lookAheadOpts = {
        paths:{
          'm/0': 10,
        },
        isWatched: true
      }
      //@ts-ignore
      const sendingKeyChain = new KeyChain({HDPublicKey: receivedContactPublicKey, lookAheadOpts });
      //@ts-ignore
      const receivingKeyChain = new KeyChain({HDPublicKey: extendedPrivateKey, lookAheadOpts });
      //@ts-ignore
      const issuedSendingKeyChainPaths = sendingKeyChain.getIssuedPaths();
      //@ts-ignore
      const issuedReceivingKeyChainPaths = receivingKeyChain.getIssuedPaths();
      establishedContact.setKeyChains({
        sending: sendingKeyChain,
        receiving: receivingKeyChain,
      });
      const chainStore = this.storage.getChainStore(this.network);

      issuedReceivingKeyChainPaths.forEach((issuedPath) => {
        chainStore.importAddress(issuedPath.address.toString());
      });
      issuedSendingKeyChainPaths.forEach((issuedPath) => {
        chainStore.importAddress(issuedPath.address.toString());
      });
    })
  return establishedContacts;
}
