class Socket {
    constructor(app){
        if(!app.hasOwnProperty('config') || !app.config.hasOwnProperty('socket')){
            throw new Error('Missing config for socket.');
        }
        if((app.config.socket.hasOwnProperty('enable') && app.config.socket.enable===false)){
            app.socket = this;
            return false;
        }
        
        app.socket = require('socket.io')();
        app.socket.on('connection', function(client){
            let host = null;
            if(client && client.hasOwnProperty('handshake') && client.handshake.hasOwnProperty('headers') && client.handshake.headers.hasOwnProperty('host')){
                host = client.handshake.headers.host;
            }
            console.log(`Client ${client.conn.id} has connected from ${host}`);

        });
        app.socket.listen(app.config.socket.port);
        console.log('Websocket started...', app.config.socket.port);
    }
}
module.exports=Socket;