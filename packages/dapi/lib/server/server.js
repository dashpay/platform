const routes = require('./routes');
const express = require('express');

class Server {
    constructor(app){
        app.server = express();
        routes.setup(app);
        app.server.listen(app.config.express.port);
        console.log('Server listening on port:', app.config.express.port);
    }
}
module.exports=Server;