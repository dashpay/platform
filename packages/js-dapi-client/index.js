const {merge} = require('khal').misc;
const Connector = require('./util/Connector');
const EE2 = require('eventemitter2').EventEmitter2;

const SDK = async function(options={}){
    let self = {};

    //TODO : Which components will be used to calculate the fees Wallet.Fees.calculate(prepareTx) ?
    //Contains some seeds like for MN or Socket.
    let defaultConfig = require('./config.js');
    self._config = merge(options,defaultConfig);
    if(options.hasOwnProperty('DISCOVER') && options.DISCOVER.hasOwnProperty('INSIGHT_SEEDS') && options.DISCOVER.INSIGHT_SEEDS.constructor.name=="Array"){
        self._config.DISCOVER.INSIGHT_SEEDS = options.DISCOVER.INSIGHT_SEEDS.concat(defaultConfig.DISCOVER.INSIGHT_SEEDS);
    }
    if(self._config.debug) process.on('unhandledRejection', r => console.log(r));

    //The Account part will be use to provide Account functionnality,
    //Therefore it will allow to connect and retrieve user information
    self.Accounts = require('./Accounts/').Accounts.call(self);

    //The Wallet part will be used to do stuff based on manipulating the Dash.
    //Data will be provided from Accounts which will store the Pub/Prv keys.
    //It will perform action such as sending a payment, analyzing tx history and stuff like this.
    //It should allow InstantSend and PrivateSend as well.
    self.Wallet = require('./Wallet/').Wallet.call(self);

    //The Discover part will be use to checkout a Masternode List (and therefore the insightAPI associated)
    //It will validate these Masternode in order to be sure to have a quorum of masternode delivering data that will follow the consensus.
    //It will also verify that theses Masternode still have the 1000 collateral
    self.Discover = require('./Discover').Discover.call(self);

    //The Explorer will be the connector with Insight-API.
    //It will for instance checkout headers from insight API based on the list of masternode from Discover
    //It will then validate the headers and store it to the Blockchain.
    self.Explorer = require('./Explorer').Explorer.call(self);

    //Blockchain is where will be stored all the blockchain information
    //This will include for exemple all the headers for exemple
    self.Blockchain = require('./Blockchain').Blockchain.call(self);

    self.CONNECTOR_TYPE="CLIENT";
    self.IS_CONNECTED=false;

    //Prepare EventEmitter
    self.emitter = new EE2();

    //Create Socket
    let socketOpened = await Connector.createSocket(self);
    if(!socketOpened){
       if(self._config.errors) console.error(`Socket - Couldn't connect to any MN`);
    }
    //Init masternode fetching
    await self.Discover.init();

    //First we restore the last Blockchain we have stored.
    //Then we fetch last one
    await self.Blockchain.init();

    //Special attachment : When receives a message with _reqId, will emit an event.
    self.socket.on('message',function(msg){
        let message = JSON.parse(msg);
        if(message.hasOwnProperty('_reqId')){
            self.emitter.emit(message._reqId, message);
        }
    })
    self.emitter.emit('ready',"ready");

    return self;
}
module.exports = SDK;
