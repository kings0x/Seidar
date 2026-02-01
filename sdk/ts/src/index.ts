export interface QuoteRequest {
  service_type: string;
  user_address: string;
  duration_seconds: number;
}

export interface QuoteResponse {
  quote: any;
  signature: string;
  hash: string;
}

export class ProxyClient {
  private proxyUrl: string;

  constructor(proxyUrl: string) {
    this.proxyUrl = proxyUrl.replace(/\/$/, "");
  }

  /**
   * Request a quote for a specific service tier.
   */
  async requestQuote(req: QuoteRequest): Promise<QuoteResponse> {
    const response = await fetch(`${this.proxyUrl}/api/v1/quote`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(req),
    });

    if (!response.ok) {
      throw new Error(`Failed to request quote: ${response.statusText}`);
    }

    return response.json();
  }

  /**
   * Perform a proxied request with the required user address header.
   */
  async proxyGet(path: string, userAddress: string): Promise<Response> {
    const formattedPath = path.startsWith("/") ? path : `/${path}`;
    return fetch(`${this.proxyUrl}${formattedPath}`, {
      method: "GET",
      headers: {
        "X-User-Address": userAddress,
      },
    });
  }
}
