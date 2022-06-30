/**
 * For provided contact request, perform an acceptance operation (by sending an opposite contact request)
 * @param contactRequest contact request document
 */
export async function acceptContactRequest(this: any, contactRequest){
  if(!contactRequest){
    throw new Error('Expecting a contact request to accept');
  }
  const senderUniqueId = contactRequest.getOwnerId();
  const [senderContactDocument] = await this.platform.names.resolveByRecord('dashUniqueIdentityId',senderUniqueId);
  if(!senderContactDocument || !senderContactDocument.data.label){
    throw new Error('Unable to accept the contact request: sender name was not found.')
  }
  return this.sendContactRequest(senderContactDocument.data.label);
}
