export class Profile {
  public avatarUrl: string;
  public displayName: string;
  public publicMessage: string;
  public ownerId: any;

  constructor(profileDocument) {
    const { avatarUrl, displayName, publicMessage } = profileDocument.data;
    this.avatarUrl = avatarUrl;
    this.displayName = displayName;
    this.publicMessage = publicMessage;
    this.ownerId = profileDocument.ownerId;
  }
}
