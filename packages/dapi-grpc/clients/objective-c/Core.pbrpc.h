#import "Core.pbobjc.h"

#import <ProtoRPC/ProtoService.h>
#import <ProtoRPC/ProtoRPC.h>
#import <RxLibrary/GRXWriteable.h>
#import <RxLibrary/GRXWriter.h>



NS_ASSUME_NONNULL_BEGIN

@protocol Core <NSObject>

#pragma mark getLastUserStateTransitionHash(LastUserStateTransitionHashRequest) returns (LastUserStateTransitionHashResponse)

- (void)getLastUserStateTransitionHashWithRequest:(LastUserStateTransitionHashRequest *)request handler:(void(^)(LastUserStateTransitionHashResponse *_Nullable response, NSError *_Nullable error))handler;

- (GRPCProtoCall *)RPCTogetLastUserStateTransitionHashWithRequest:(LastUserStateTransitionHashRequest *)request handler:(void(^)(LastUserStateTransitionHashResponse *_Nullable response, NSError *_Nullable error))handler;


#pragma mark subscribeToBlockHeadersWithChainLocks(BlockHeadersWithChainLocksRequest) returns (stream BlockHeadersWithChainLocksResponse)

- (void)subscribeToBlockHeadersWithChainLocksWithRequest:(BlockHeadersWithChainLocksRequest *)request eventHandler:(void(^)(BOOL done, BlockHeadersWithChainLocksResponse *_Nullable response, NSError *_Nullable error))eventHandler;

- (GRPCProtoCall *)RPCTosubscribeToBlockHeadersWithChainLocksWithRequest:(BlockHeadersWithChainLocksRequest *)request eventHandler:(void(^)(BOOL done, BlockHeadersWithChainLocksResponse *_Nullable response, NSError *_Nullable error))eventHandler;


@end

/**
 * Basic service implementation, over gRPC, that only does
 * marshalling and parsing.
 */
@interface Core : GRPCProtoService<Core>
- (instancetype)initWithHost:(NSString *)host NS_DESIGNATED_INITIALIZER;
+ (instancetype)serviceWithHost:(NSString *)host;
@end

NS_ASSUME_NONNULL_END
