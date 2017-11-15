const message = require('bitcore-message-dash');
const qDash = require('quorums-dash')

class Quorum {
	constructor(app) {
		this.logger = app.logger;
		this.logger.debug('- Init Quorum');
		this.insight = app.insight;
	}

	getQuorumHash() {
		return new Promise((resolve, reject) => {
			this.insight.getCurrentBlockHeight()
				.then(height => {
					return this.insight.getHashFromHeight(qDash.getRefHeight(height))
				})
				.then(hash => {
					resolve(hash)
				})
		})
	}

	isValidQuorum(body, qTempPort) {

		return new Promise((resolve, reject) => {
			Promise.all([this.insight.getMnList(), this.getQuorumHash(), this.insight.getAddress(body.data.txId)])
				.then(([list, hash, addr]) => {
					let quorumData = {
						mnList: list,
						refHash: hash,
						refAddr: addr
					}

					resolve(qDash.validate(body.data, body.signature, quorumData, qTempPort)) ////QDEVTEMP - remove qTempPort
				})
		})

	}

	getQuroumFailedResponse() {
		this.logger.debug('Invalid Quorum! - Signature invalid or invalid node for handling request ');
		return { "response": "Failed" }
	}

	performAction(type, val) {
		this.logger.debug('Quorum - Received action ', type, val);
		switch (type) {
			case "add":
				return this.addObject(val);
				break;
			case "commit":
				return this.commitObject(val);
				break;
			case "remove":
				return this.removeObject(val);
				break;
			case "state":
				return this.getState(val);
				break;
			case "listen":
				return this.listenForeignKey(val);
				break;
			case "migrate":
				return this.migrateState(val);
				break;
			case "auth":
				return this.authenticate(val);
				break;
			default:
				return "Not Implemented - PerformAction " + type;
		}
	}
	addObject(value) { return { "response": "Added" }; }
	commitObject(value) { return { "response": "Commited" }; }
	removeObject(value) { return { "response": "Removed" }; }
	getState(value) { return { "response": "Getted" }; }
	listenForeignKey(value) { return { "response": "Listened" }; }
	migrateState(value) { return { "response": "Migrated" }; }
	authenticate(value) { return { "response": "Authenticated" }; }
};
module.exports = Quorum;