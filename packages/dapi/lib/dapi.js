const AuthService = require('./authService/authService');
const Server = require('./server/server');
class Dapi {
    constructor(config) {
        this.config = config;
        this.authService = new AuthService(this);

        this.server = new Server(this);
    }

}
module.exports = Dapi;
