import { ChannelCredentials } from "npm:@grpc/grpc-js";
export { TokenSetClient, UserSetClient } from "./codegen/proto/backend.ts";

export async function generateCredential(): Promise<ChannelCredentials> {
  const clientCert = await Deno.readFile("../cert/cert.pem");
  const clientKey = await Deno.readFile("../cert/key.pem");

  const channelCredentials = ChannelCredentials.createSsl(
    undefined,
    clientKey,
    clientCert,
  );

  return channelCredentials;
}

export const serverAddress = "127.0.0.1:8080";
