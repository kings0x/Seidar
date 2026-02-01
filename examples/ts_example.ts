import { ProxyClient } from "../sdk/ts/src";

async function run() {
    const client = new ProxyClient("http://localhost:8080");

    const address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";

    try {
        // 1. Request a Quote
        console.log("Requesting quote...");
        const quote = await client.requestQuote({
            service_type: "tier_2",
            user_address: address,
            duration_seconds: 86400,
        });
        console.log("Quote received:", quote.hash);

        // 2. Perform a proxied request
        console.log("Performing proxied request...");
        const response = await client.proxyGet("/health", address);
        console.log("Response Status:", response.status);
    } catch (error) {
        console.error("Error:", error);
    }
}

run();
