export const config = { runtime: "edge" };

export default async function (): Promise<Response> {
  return new Response("ok");
}
