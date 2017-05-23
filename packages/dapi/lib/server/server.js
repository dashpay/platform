const routes = require('./routes');
const express = require('express');

class Server {
    constructor(app) {
        if (!app.hasOwnProperty('config') || !app.config.hasOwnProperty('server')) {
            throw new Error('Missing config for server.');
        }
        if (!app.hasOwnProperty('config') || !app.config.hasOwnProperty('server') || (app.config.server.hasOwnProperty('enable') && app.config.server.enable === false)) {
            app.server = null;
            return false;
        }
        app.server = express();
        routes.setup(app);
        app.server.listen(app.config.server.port);
        console.log('Server listening on port:', app.config.server.port);


    }
}
module.exports = Server;