#import "Core.pbrpc.h"

#import <ProtoRPC/ProtoRPC.h>
#import <RxLibrary/GRXWriter+Immediate.h>

@implementation Core

// Designated initializer
- (instancetype)initWithHost:(NSString *)host {
  return (self = [super initWithHost:host packageName:@"org.dash.platform.dapi.v0" serviceName:@"Core"]);
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


#pragma mark getStatus(GetStatusRequest) returns (GetStatusRequest)

- (void)getStatusWithRequest:(GetStatusRequest *)request handler:(void(^)(GetStatusRequest *_Nullable response, NSError *_Nullable error))handler{
  [[self RPCTogetStatusWithRequest:request handler:handler] start];
}
// Returns a not-yet-started RPC object.
- (GRPCProtoCall *)RPCTogetStatusWithRequest:(GetStatusRequest *)request handler:(void(^)(GetStatusRequest *_Nullable response, NSError *_Nullable error))handler{
  return [self RPCToMethod:@"getStatus"
            requestsWriter:[GRXWriter writerWithValue:request]
             responseClass:[GetStatusRequest class]
        responsesWriteable:[GRXWriteable writeableWithSingleHandler:handler]];
}
#pragma mark getBlock(GetBlockRequest) returns (GetBlockResponse)

- (void)getBlockWithRequest:(GetBlockRequest *)request handler:(void(^)(GetBlockResponse *_Nullable response, NSError *_Nullable error))handler{
  [[self RPCTogetBlockWithRequest:request handler:handler] start];
}
// Returns a not-yet-started RPC object.
- (GRPCProtoCall *)RPCTogetBlockWithRequest:(GetBlockRequest *)request handler:(void(^)(GetBlockResponse *_Nullable response, NSError *_Nullable error))handler{
  return [self RPCToMethod:@"getBlock"
            requestsWriter:[GRXWriter writerWithValue:request]
             responseClass:[GetBlockResponse class]
        responsesWriteable:[GRXWriteable writeableWithSingleHandler:handler]];
}
#pragma mark sendTransaction(SendTransactionRequest) returns (SendTransactionResponse)

- (void)sendTransactionWithRequest:(SendTransactionRequest *)request handler:(void(^)(SendTransactionResponse *_Nullable response, NSError *_Nullable error))handler{
  [[self RPCTosendTransactionWithRequest:request handler:handler] start];
}
// Returns a not-yet-started RPC object.
- (GRPCProtoCall *)RPCTosendTransactionWithRequest:(SendTransactionRequest *)request handler:(void(^)(SendTransactionResponse *_Nullable response, NSError *_Nullable error))handler{
  return [self RPCToMethod:@"sendTransaction"
            requestsWriter:[GRXWriter writerWithValue:request]
             responseClass:[SendTransactionResponse class]
        responsesWriteable:[GRXWriteable writeableWithSingleHandler:handler]];
}
#pragma mark getTransaction(GetTransactionRequest) returns (GetTransactionResponse)

- (void)getTransactionWithRequest:(GetTransactionRequest *)request handler:(void(^)(GetTransactionResponse *_Nullable response, NSError *_Nullable error))handler{
  [[self RPCTogetTransactionWithRequest:request handler:handler] start];
}
// Returns a not-yet-started RPC object.
- (GRPCProtoCall *)RPCTogetTransactionWithRequest:(GetTransactionRequest *)request handler:(void(^)(GetTransactionResponse *_Nullable response, NSError *_Nullable error))handler{
  return [self RPCToMethod:@"getTransaction"
            requestsWriter:[GRXWriter writerWithValue:request]
             responseClass:[GetTransactionResponse class]
        responsesWriteable:[GRXWriteable writeableWithSingleHandler:handler]];
}
#pragma mark getEstimatedTransactionFee(GetEstimatedTransactionFeeRequest) returns (GetEstimatedTransactionFeeResponse)

- (void)getEstimatedTransactionFeeWithRequest:(GetEstimatedTransactionFeeRequest *)request handler:(void(^)(GetEstimatedTransactionFeeResponse *_Nullable response, NSError *_Nullable error))handler{
  [[self RPCTogetEstimatedTransactionFeeWithRequest:request handler:handler] start];
}
// Returns a not-yet-started RPC object.
- (GRPCProtoCall *)RPCTogetEstimatedTransactionFeeWithRequest:(GetEstimatedTransactionFeeRequest *)request handler:(void(^)(GetEstimatedTransactionFeeResponse *_Nullable response, NSError *_Nullable error))handler{
  return [self RPCToMethod:@"getEstimatedTransactionFee"
            requestsWriter:[GRXWriter writerWithValue:request]
             responseClass:[GetEstimatedTransactionFeeResponse class]
        responsesWriteable:[GRXWriteable writeableWithSingleHandler:handler]];
}
#pragma mark subscribeToBlockHeadersWithChainLocks(BlockHeadersWithChainLocksRequest) returns (stream BlockHeadersWithChainLocksResponse)

- (void)subscribeToBlockHeadersWithChainLocksWithRequest:(BlockHeadersWithChainLocksRequest *)request eventHandler:(void(^)(BOOL done, BlockHeadersWithChainLocksResponse *_Nullable response, NSError *_Nullable error))eventHandler{
  [[self RPCTosubscribeToBlockHeadersWithChainLocksWithRequest:request eventHandler:eventHandler] start];
}
// Returns a not-yet-started RPC object.
- (GRPCProtoCall *)RPCTosubscribeToBlockHeadersWithChainLocksWithRequest:(BlockHeadersWithChainLocksRequest *)request eventHandler:(void(^)(BOOL done, BlockHeadersWithChainLocksResponse *_Nullable response, NSError *_Nullable error))eventHandler{
  return [self RPCToMethod:@"subscribeToBlockHeadersWithChainLocks"
            requestsWriter:[GRXWriter writerWithValue:request]
             responseClass:[BlockHeadersWithChainLocksResponse class]
        responsesWriteable:[GRXWriteable writeableWithEventHandler:eventHandler]];
}
@end
