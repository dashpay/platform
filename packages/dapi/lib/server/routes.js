const Handlers = require('./handlers');
const Routes = {
    setup: function(app) {
        let handlers = new Handlers(app);
        app.server.get('/', handlers.getHello);
        app.server.get('/blockcount', handlers.getBlockCount);
        app.server.get('/getinfo', handlers.getInfo);
    }
};
module.exports = Routes;