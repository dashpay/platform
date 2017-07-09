class Requester extends require('./receive_socket') {
    constructor(params) {
        super(params);
    }

    attach() {
        super.attach('req');
    }
}

module.exports = Requester;