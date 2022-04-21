export class Contact {
  public username?: null|string;
  public identityId: any;
  public sentRequest: any;
  public receivedRequest: any;
  public identity?: any;
  public profile?: any;
  public keys?: any;
  public keychains?: any;

  constructor(identityId, sentRequest, receivedRequest) {
    this.username = null;
    this.identity = null;
    this.identityId = identityId;
    this.sentRequest = sentRequest;
    this.receivedRequest = receivedRequest;
  }
  setUsername(username){
    this.username = username;
  }
  setIdentity(identity){
    this.identity = identity;
  }
  setProfile(profile){
    this.profile = profile
  }
  setHDKeys(keysSet){
    const { receiving, sending } = keysSet;
    this.keys = {
      receiving,
      sending
    };
  }
  setKeyChains(keychainsSet){
    const { receiving, sending } = keychainsSet;
    this.keychains = {
      receiving,
      sending
    };
  }
  getUnusedAddress(type = 'receiving'){
    const keychain = this.keychains[type];
    return keychain.getFirstUnusedAddress();
  }
}
