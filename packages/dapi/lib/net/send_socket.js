class SendSocket extends require('./socketbase') {
    constructor(params) {
        super(params)
    }

    attach(socketType) {

        super.attach(socketType)
        this.socket.on('accept', function(fileDescriptor, replierEndpoint) {
            console.log(`${this.constructor.name} - Node (${fileDescriptor}) Connected`, replierEndpoint)
        })

        this.socket.on('listen', function(fileDescriptor, replierEndpoint) {
            console.log(`${this.constructor.name} - Listening for new connection:`)
        })
        this.socket.bind(this.uri);
    }
}

module.exports = SendSocket