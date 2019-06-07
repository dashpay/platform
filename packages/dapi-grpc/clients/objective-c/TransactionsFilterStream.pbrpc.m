#import "TransactionsFilterStream.pbrpc.h"

#import <ProtoRPC/ProtoRPC.h>
#import <RxLibrary/GRXWriter+Immediate.h>

@implementation TransactionsFilterStream

// Designated initializer
- (instancetype)initWithHost:(NSString *)host {
  return (self = [super initWithHost:host packageName:@"org.dash.platform.dapi" serviceName:@"TransactionsFilterStream"]);
}

// Override superclass initializer to disallow different package and service names.
- (instancetype)initWithHost:(NSString *)host
                 packageName:(NSString *)packageName
                 serviceName:(NSString *)serviceName {
  return [self initWithHost:host];
}

+ (instancetype)serviceWithHost:(NSString *)host {
  return [[self alloc] initWithHost:host];
}


#pragma mark subscribeToTransactionsWithProofs(TransactionsWithProofsRequest) returns (stream TransactionsWithProofsResponse)

- (void)subscribeToTransactionsWithProofsWithRequest:(TransactionsWithProofsRequest *)request eventHandler:(void(^)(BOOL done, TransactionsWithProofsResponse *_Nullable response, NSError *_Nullable error))eventHandler{
  [[self RPCTosubscribeToTransactionsWithProofsWithRequest:request eventHandler:eventHandler] start];
}
// Returns a not-yet-started RPC object.
- (GRPCProtoCall *)RPCTosubscribeToTransactionsWithProofsWithRequest:(TransactionsWithProofsRequest *)request eventHandler:(void(^)(BOOL done, TransactionsWithProofsResponse *_Nullable response, NSError *_Nullable error))eventHandler{
  return [self RPCToMethod:@"subscribeToTransactionsWithProofs"
            requestsWriter:[GRXWriter writerWithValue:request]
             responseClass:[TransactionsWithProofsResponse class]
        responsesWriteable:[GRXWriteable writeableWithEventHandler:eventHandler]];
}
@end
