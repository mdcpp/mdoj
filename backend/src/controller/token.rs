use openssl;

fn verify_token(token: string) {
    // token:'{user primary key in decimal(10 len)}{token salt in 32 len string}'
    // encrypt token by AES
}
