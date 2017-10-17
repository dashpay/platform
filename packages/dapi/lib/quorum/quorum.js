const message = require('bitcore-message-dash');
const quorumManager = require('./quorummanager')

class Quorum {
	constructor(app) {
		this.logger = app.logger;

		this.logger.debug('- Init Quorum');
	}

	//Todo: temp insight calls to insight until rpc
	userIsInQuorum(insight, txId) {

		return new Promise(function(resolve, reject) {

			let tmpMnList = []; //used to resolve vin from ip while rpc implementation is pending

			Promise.all([insight.getMnList(), insight.getLastBlockHash()])
				.then(r => {
					tmpMnList = r[0]
					return quorumManager.getQuorum(r[0], r[1], txId);
				})
				.then(quorum => {
					let vin = quorumManager.resolveVinFromIp(tmpMnList, quorum.ip); //temp until rpc to get outpoint

					resolve(true);
				})
		})
	}
	validate(data, signature, insight) {
		let self = this;
		return new Promise(function(resolve, reject) {
			insight.getAddress(data.txId)
				.then(addr => {
					if (!message(data.toString()).verify(addr, signature)) {
						return false;
					}
					else {
						return true;
					}
				})
				.then(signatureValid => {
					if (signatureValid) {
						return self.userIsInQuorum(insight, data.txId)
					}
					else {
						this.logger.debug('Invalid signature');
						return false;
					}
				})
				.then(inQuorum => {
					if (inQuorum) {
						resolve(true);
					}
					else {
						resolve(false);
					}
				}).catch(ex => {
					console.log(ex);
				})
		})
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