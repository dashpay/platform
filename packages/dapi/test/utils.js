'use strict'
const should = require('should');
const {Logger}=require('../lib/utils');

describe('Utils - Utils', function () {
	let logger = new Logger();
	let logger2 = new Logger('test.log');
	it('should be able to start a logger', function () {
		//test logger
		//test logger2 by checking file exist.
	});
	//How to test stdout ?
	it('should do stuff...',function () {
		function log(){
			logger.log("Test d'un log");

			logger.log('fatal','This is a specified fatal thing!');
			logger.fatal('This is a fatal thing!');

			logger.log('error','This is an error (log/err)');
			logger.error('This is also an error');

			logger.log('warn','This is a warning');
			logger.warn('This is also a warning');

			logger.log('notice','This is a important simple information');
			logger.notice('This is also a important simple information');

			logger.log('info','This is a simple information');
			logger.info('This is also a simple information');

			logger.log('debug','This is a debug thing');
			logger.debug('This is also a debug thing');

			logger.log('verbose','This is a verbose useless thing');
			logger.verbose('This is also a verbose useless thing');
		}
		console.log('---- DEFAULT');
		log();
		console.log('---- 0 FATAL');
		logger.level = logger.FATAL;
		log()//By default it will do that as 'standard' level.
		console.log('---- 1 ERROR');
		logger.level = logger.ERROR;
		log()
		console.log('---- 2 WARN');
		logger.level = logger.WARN;
		log()
		console.log('---- 3 NOTICE');
		logger.level = logger.NOTICE;
		log()
		console.log('---- 4 INFO');
		logger.level = logger.INFO;
		log()
		console.log('---- 5 DEBUG');
		logger.level = logger.DEBUG;
		log();
		console.log('---- 6 VERBOSE');
		logger.level = logger.VERBOSE;
		log()
	})
}); 
