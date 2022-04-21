import { Profile } from "../types/Profile";

/**
 *
 */
export async function fetchProfile(this: any, identity){
  const identityId = (identity.getId) ? identity.getId : identity;
  const rawDocuments = await this.platform.documents.get('dashpay.profile', {
    where: [
      ['$ownerId', '==', identityId],
    ],
  });

  const [profileDocument] = rawDocuments;
  return new Profile(profileDocument);
}
