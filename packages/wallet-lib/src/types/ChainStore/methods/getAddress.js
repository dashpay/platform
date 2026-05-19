function getAddress(address) {
  return this.state.addresses.get(address.toString());
}

export default getAddress;