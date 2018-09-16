class DAPIClient {
  constructor() {
    this.type = this.constructor.name;
  }

  async getAddressSummary() {
    console.error(`Missing implementation - ${this.type} - getAddressSummary`);
    return false;
  }

  async getStatus() {
    console.error(`Missing implementation - ${this.type} - getStatus`);
    return false;
  }

  async getTransaction() {
    console.error(`Missing implementation - ${this.type} - getTransaction`);
    return false;
  }

  async getUTXO() {
    console.error(`Missing implementation - ${this.type} - getUTXO`);
    return false;
  }

  async subscribeToAddresses() {
    console.error(`Missing implementation - ${this.type} - subscribeToAddresses`);
    return false;
  }

  async subscribeToEvent() {
    console.error(`Missing implementation - ${this.type} - subscribeToEvent`);
    return false;
  }

  async unsubscribeFromEvent() {
    console.error(`Missing implementation - ${this.type} - unsubscribeFromEvent`);
    return false;
  }

  async sendRawTransaction() {
    console.error(`Missing implementation - ${this.type} - sendRawTransaction`);
    return false;
  }

  async updateNetwork() {
    console.error(`Missing implementation - ${this.type} - updateNetwork`);
    return false;
  }

  async closeSocket() {
    console.error(`Missing implementation - ${this.type} - closeSocket`);
    return false;
  }
}
module.exports = DAPIClient;
