const {
  User, SubTx, Transition, State,

} = require('@dashevo/dash-schema/lib').Consensus;

const jayson = require('jayson');
const log = console;

const mockedData = {
  user: {
    uname: '',
    regtxid: 'ef6ab42e001144bfbaf4777b05148f56a9705b63cdc320c95171bc600df7088e',
    pubkey: '024964f06ea5cfec1890d7e526653b083c12360f79164c1e8163327d0849fa7bca',
    credits: 100000,
    subtx: [],
  },
  transition: {},
};

// All methods are async because when we remove mocks there will be network calls
const dashrpc = {
  async getUser(username) {
    if (!User.validateUsername(username)) {
      throw new Error('Username is not valid');
    }
    const user = Object.assign({}, mockedData.user);
    user.uname = username;
    return user;
  },
  async createRawSubTx(userData) {
    if (!User.validateUser(userData)) {
      throw new Error('User data is not valid');
    }
    return mockedData.user.regtxid;
  },
  async sendRawTransaction(serializedTransaction) {
    const txId = 'some_txid_provided_by_dashd';
    return txId;
  },
};

const logic = {
  /**
   * This method should do quorum validation
   * @param transitionData
   * @returns {Promise.<string>}
   */
  async sendTransition(transitionData) {
    if (!Transition.validate(transitionData)) {
      throw new Error('Transition data is not valid');
    }
    const transition = Object.assign({}, transitionData);
    transition.qsig = '1';
    return State.getTSID(transition);
  },
  async sendRawSubtx(transactionData) {
    if (!SubTx.validate(transactionData)) {
      throw new Error('SubTx data is not valid');
    }
    return State.getTSID(transactionData);
  },
};

const server = jayson.server({
  /**
   * Returns user
   * @param args
   * @param callback
   */
  async getUser(args, callback) {
    const username = args[0];
    try {
      const user = await dashrpc.getUser(username);
      // We need transition header
      // If we do not have any transitions just do not return last transition header
      return callback(null, user);
    } catch (e) { return callback(e); }
  },
  /**
   * Raw subscription transaction that need to be signed by the client.
   * When client de-serialized and signed transaction,
   * client needs to serialize it again and call sendRawTransaction method
   * @param args
   * @param callback
   */
  async createRawSubTx(args, callback) {
    const user = {
      data: Object.assign({}, args),
      objtype: 'User',
    };
    try {
      const subTx = await dashrpc.createRawSubTx(user);
      return callback(null, subTx);
    } catch (e) {
      log.error(e);
      return callback(e);
    }
  },
  /**
   * Passes signed transaction to dashd
   * @param args
   * @param callback
   */
  async sendRawTransaction(args, callback) {
    const signedTransaction = args[0];
    try {
      const subTxId = await dashrpc.sendRawTransaction(signedTransaction);
      return callback(null, subTxId);
    } catch (e) {
      log.error(e);
      return callback(e);
    }
  },
  async sendTransition(args, callback) {
    const transitionData = args.data;
    try {
      const transitionId = await logic.sendTransition(transitionData);
      callback(null, transitionId);
    } catch (e) {
      log.error(e);
      callback(e);
    }
  },
});

const port = 4019;

server.http().listen(port);
console.log(`RPC server is listening on port ${port}`);

