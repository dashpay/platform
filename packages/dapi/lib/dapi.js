const AuthService = require('./authService/authService');
const Insight = require('./insight/insight');
const Server = require('./server/server');
const Quorum = require('./quorum/quorum');
const {Logger} = require('./utils');

class Dapi {
    constructor(config) {
        this.logger = new Logger();
        this.logger.level=this.logger.VERBOSE;
        this.config = config;
        
        this.insight = new Insight(this);
        this.quorum = new Quorum(this);
        
        this.authService = new AuthService(this);
        this.server = new Server(this);
    }

}
module.exports = Dapi;
