const User = require('dash-schema/lib').Consensus.User;

const mockedData = {
  user: {

  },
};

// All methods are async because when we remove mocks there will be network calls
const dashrpc = {
  async getUser(username) {
    if (User.validateUsername(username)) {
      throw new Error('Username is not valid');
    }
    const user = Object.assign({}, mockedData.user);
    return user;
  },
};

module.exports = dashrpc;
