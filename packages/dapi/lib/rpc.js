const {
  User, SubTx, Transition, State,
} = require('@dashevo/dash-schema/lib').Consensus;
const jayson = require('jayson');

const config = require('./config');
const insight = require('./insight');

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
  async createRawSubTx(userData) {
    if (!User.validateUser(userData)) {
      throw new Error('User data is not valid');
    }
    return mockedData.user.regtxid;
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
    if (!User.validateUsername(username)) {
      return callback({ code: 400, message: 'Username is not valid' });
    }
    try {
      const user = await insight.getUser(username);
      // TODO: We need transition header
      // If we do not have any transitions just do not return last transition header
      return callback(null, user);
    } catch (e) {
      log.error(e.stack);
      return callback({ code: 400, message: e.message });
    }
  },
  /**
   * Passes signed transaction to dashd. Transaction must be constructed and signed on client side.
   * @param args
   * @param callback
   */
  async sendRawTransaction(args, callback) {
    const signedTransaction = args[0];
    try {
      const subTxId = await insight.sendRawTransaction(signedTransaction);
      return callback(null, subTxId);
    } catch (e) {
      log.error(e.stack);
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
log.info(`RPC is running in ${config.name} mode`);
log.info(`Insight uri is ${config.insightUri}`);
log.info(`RPC server is listening on port ${port}`);

