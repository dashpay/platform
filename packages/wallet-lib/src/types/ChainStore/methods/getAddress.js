function getAddress(address) {
  return this.state.addresses.get(address.toString());
}

module.exports = getAddress;
