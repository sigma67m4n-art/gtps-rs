#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include <enet/enet.h>

static uint32_t gtps_rand_state = 0;

uint32_t gtps_enet_rand(void)
{
	if (gtps_rand_state == 0) {
		gtps_rand_state = (uint32_t) enet_time_get() ^ 0x9E3779B9u;
		if (gtps_rand_state == 0)
			gtps_rand_state = 0x1234567u;
	}

	gtps_rand_state ^= gtps_rand_state << 13;
	gtps_rand_state ^= gtps_rand_state >> 17;
	gtps_rand_state ^= gtps_rand_state << 5;

	return gtps_rand_state;
}

static int gtps_configure(ENetHost *host)
{
	if (host == NULL)
		return -1;

	host->checksum = enet_crc32;

	return enet_host_compress_with_range_coder(host);
}

ENetHost *gtps_enet_host_create_server(const ENetAddress *address, size_t peers, size_t channels)
{
	ENetHost *host = enet_host_create(address, peers, channels, 0, 0);

	if (host == NULL)
		return NULL;

	host->usingNewPacketForServer = 1;

	if (gtps_configure(host) != 0) {
		enet_host_destroy(host);
		return NULL;
	}

	return host;
}

ENetHost *gtps_enet_host_create_client(const ENetAddress *address, size_t peers, size_t channels)
{
	ENetHost *host = enet_host_create(address, peers, channels, 0, 0);

	if (host == NULL)
		return NULL;

	host->usingNewPacket = 1;

	if (gtps_configure(host) != 0) {
		enet_host_destroy(host);
		return NULL;
	}

	return host;
}

int gtps_enet_address_set_ip(ENetAddress *address, const char *ip)
{
	return enet_address_set_host_ip(address, ip);
}

void gtps_enet_address_set_port(ENetAddress *address, uint16_t port)
{
	address->port = port;
}

int gtps_enet_host_service(ENetHost *host, void *event, uint32_t timeout_ms)
{
	return enet_host_service(host, (ENetEvent *) event, timeout_ms);
}

int gtps_enet_event_type(const void *event)
{
	return (int) ((const ENetEvent *) event)->type;
}

uint32_t gtps_enet_event_data(const void *event)
{
	return (uint32_t) ((const ENetEvent *) event)->data;
}

uint8_t gtps_enet_event_channel(const void *event)
{
	return ((const ENetEvent *) event)->channelID;
}

void *gtps_enet_event_peer(const void *event)
{
	return ((const ENetEvent *) event)->peer;
}

void *gtps_enet_event_packet(const void *event)
{
	return ((const ENetEvent *) event)->packet;
}

int gtps_enet_peer_index(const ENetHost *host, const ENetPeer *peer)
{
	if (host == NULL || peer == NULL)
		return -1;

	return (int) (peer - host->peers);
}

ENetPeer *gtps_enet_host_peer(ENetHost *host, uint32_t id)
{
	if (host == NULL || id >= host->peerCount)
		return NULL;

	return &host->peers[id];
}

size_t gtps_enet_host_connected_peers(const ENetHost *host)
{
	return host->connectedPeers;
}

int gtps_enet_peer_state(const ENetPeer *peer)
{
	return (int) peer->state;
}

uint16_t gtps_enet_peer_port(const ENetPeer *peer)
{
	return peer->address.port;
}

int gtps_enet_peer_ip(const ENetPeer *peer, char *buffer, size_t buffer_length)
{
	return enet_address_get_host_ip(&peer->address, buffer, buffer_length);
}

uint32_t gtps_enet_peer_rtt(const ENetPeer *peer)
{
	return peer->roundTripTime;
}

int gtps_enet_peer_send(ENetPeer *peer, uint8_t channel, ENetPacket *packet)
{
	return enet_peer_send(peer, channel, packet);
}

void gtps_enet_peer_disconnect(ENetPeer *peer, uint32_t data)
{
	enet_peer_disconnect(peer, data);
}

void gtps_enet_peer_disconnect_now(ENetPeer *peer, uint32_t data)
{
	enet_peer_disconnect_now(peer, data);
}

void gtps_enet_peer_reset(ENetPeer *peer)
{
	enet_peer_reset(peer);
}

const uint8_t *gtps_enet_packet_data(const ENetPacket *packet)
{
	return packet->data;
}

size_t gtps_enet_packet_size(const ENetPacket *packet)
{
	return packet->dataLength;
}

void gtps_enet_packet_destroy(ENetPacket *packet)
{
	enet_packet_destroy(packet);
}
