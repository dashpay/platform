const AuthService = require('./authService/authService');
const Server = require('./server/server');
const RPC = require('./rpc/rpc');
const ZMQ = require('./zmq/zmq');
const Socket = require('./socket/socket');
class Dapi {
    constructor(config) {
        this.config = config;
        this.rpc = new RPC(this);
        this.authService = new AuthService(this);
                
        this.zmq = new ZMQ(this);
        this.server=new Server(this);
        this.socket = new Socket(this);
    }

}
module.exports = Dapi;