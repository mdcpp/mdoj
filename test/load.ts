import { ChannelCredentials } from 'npm:@grpc/grpc-js';

import grpc from "npm:@grpc/grpc-js";
import protoLoader from "npm:@grpc/proto-loader";

export default async function () {
  let packageDefinition = protoLoader.loadSync("../proto/backend.proto", {
    keepCase: true,
    longs: String,
    enums: String,
    defaults: true,
    oneofs: true,
  });
  let protoDescriptor = grpc.loadPackageDefinition(packageDefinition);

  let backendCert=await Deno.readFile("../cert/cert.pem");
  let backendCredential=ChannelCredentials.createSsl(backendCert);

  return new protoDescriptor.routeguide.RouteGuide('localhost:8080', backendCredential);
}
