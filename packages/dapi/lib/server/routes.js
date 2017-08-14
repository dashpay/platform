const Handlers = require('./handlers');
const Routes = {
	setup: function(app) {
		let handlers = new Handlers(app);

		app.server.get('/', handlers.get.hello);
	    /*
			Quorum entry point.
			verb : - add, commit, remove, state, listen, migrate, auth.
			qid : Quorum ID of a user (based on masterblock)
			data : hexData. 
		 */
		app.server.post('/quorum', handlers.post.quorum);

		app.server.get('/blocks', handlers.get.blocks);
		app.server.get('/block/:hash', handlers.get.blockHash);
		app.server.get('/block-index/:height', handlers.get.blockHeight);
		app.server.get('/rawblock/:blockHash', handlers.get.rawBlock);

		app.server.get('/tx/:txid', handlers.get.tx.get);
		app.server.post('/tx/send', handlers.post.tx.send);
		app.server.post('/tx/sendix', handlers.post.tx.sendix);

		app.server.get('/addr/:addr', handlers.get.address.get);
		app.server.get('/addr/:addr/balance', handlers.get.address.balance);
		app.server.get('/addr/:addr/totalReceived', handlers.get.address.totalReceived);
		app.server.get('/addr/:addr/totalSent', handlers.get.address.totalSent);
		app.server.get('/addr/:addr/unconfirmedBalance', handlers.get.address.unconfirmedBalance);
		app.server.get('/addr/:addr/utxo', handlers.get.address.utxo);
		app.server.get('/addrs/:addrs/utxo', handlers.get.address.utxos);
		app.server.get('/addrs/:addrs/txs', handlers.get.address.txs);

		app.server.get('/auth/challenge/:identifier', handlers.get.auth.getChallenge)

		app.server.get('/utils/estimatefee', handlers.get.utils.estimatefee);

		app.server.get('/currency', handlers.get.currency);
		app.server.get('/status', handlers.get.status);
		app.server.get('/sync', handlers.get.sync);
		app.server.get('/peer', handlers.get.peer);
		app.server.get('/version', handlers.get.version);

		app.server.get('/masternodes/list', handlers.get.mnList);
	}
};
module.exports = Routes;