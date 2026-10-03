#!/bin/bash
# Seed a throwaway Kafka with realistic demo data for tapes/demo.tape.
# Start Kafka first (single node, plaintext on :9092):
#   docker run -d --name kitz-demo -p 9092:9092 apache/kafka:3.9.1
# then: CONTAINER=kitz-demo tapes/seed-demo.sh   (uses `docker`; set ENGINE=podman)
# WARNING: deletes every non-internal topic and consumer group on that broker.
set -euo pipefail
ENGINE=${ENGINE:-docker}
C=${CONTAINER:-kitz-demo}
k() { "$ENGINE" exec -i "$C" /opt/kafka/bin/"$@" --bootstrap-server localhost:9092; }

# Clean slate: a restart reliably stops any earlier demo traffic (producers
# started with `exec -d`), then drop every topic and group.
"$ENGINE" restart "$C" >/dev/null
until k kafka-topics.sh --list >/dev/null 2>&1; do sleep 1; done
for t in $(k kafka-topics.sh --list | grep -v '^__'); do k kafka-topics.sh --delete --topic "$t"; done
# Groups whose consumers just died stay "non-empty" until their session
# expires (~45s), so keep retrying until they're all gone.
for _ in $(seq 1 30); do
  groups=$(k kafka-consumer-groups.sh --list)
  [ -z "$groups" ] && break
  for g in $groups; do k kafka-consumer-groups.sh --delete --group "$g" >/dev/null 2>&1 || true; done
  sleep 3
done

topic() { k kafka-topics.sh --create --topic "$1" --partitions "$2" >/dev/null; }
topic orders.created 6; topic payments.settled 3; topic inventory.updates 4
topic user.signups 2; topic shipments.dispatched 3; topic audit.log 1; topic notifications.email 2

produce() { k kafka-console-producer.sh --topic "$1" --property parse.key=true --property key.separator='|' >/dev/null; }
seq 1 1200 | awk '{printf "order-%d|{\"order_id\":\"ord_%06d\",\"customer\":\"cus_%04d\",\"total\":%.2f,\"currency\":\"EUR\",\"items\":%d}\n",$1%97,$1,$1%311,($1*7.31)%400+5,$1%4+1}' | produce orders.created
seq 1 2000 | awk '{printf "sku-%d|{\"sku\":\"SKU-%05d\",\"warehouse\":\"%s\",\"delta\":%d}\n",$1%50,$1%500,($1%3?"FRA-1":"AMS-2"),($1%7)-3}' | produce inventory.updates
seq 1 300 | awk '{printf "user-%d|{\"user_id\":\"usr_%05d\",\"plan\":\"%s\",\"country\":\"%s\"}\n",$1,$1,($1%5?"free":"pro"),($1%2?"DE":"NL")}' | produce user.signups
seq 1 800 | awk '{printf "ord_%06d|{\"order_id\":\"ord_%06d\",\"carrier\":\"%s\",\"eta_days\":%d}\n",$1,$1,($1%2?"DHL":"PostNL"),$1%4+1}' | produce shipments.dispatched
seq 1 5000 | awk '{printf "svc|%s user=usr_%05d action=%s\n",($1%9?"INFO":"WARN"),$1%400,($1%3?"login":"export")}' | produce audit.log
seq 1 400 | awk '{printf "usr_%05d|{\"template\":\"welcome\",\"to\":\"usr_%05d\"}\n",$1,$1}' | produce notifications.email

# Consumer groups: one idle with a backlog, one fully caught up.
k kafka-console-consumer.sh --topic orders.created --group analytics-etl --max-messages 700 --from-beginning >/dev/null 2>&1
k kafka-console-consumer.sh --topic notifications.email --group email-sender --max-messages 400 --from-beginning >/dev/null 2>&1

# Live traffic + two live consumers (run in the background inside the container).
"$ENGINE" exec -d "$C" bash -c 'i=1300; while true; do i=$((i+1)); echo "order-$((i%97))|{\"order_id\":\"ord_$i\",\"total\":$((i%300+9)).50,\"currency\":\"EUR\"}"; sleep 0.12; done | /opt/kafka/bin/kafka-console-producer.sh --bootstrap-server localhost:9092 --topic orders.created --property parse.key=true --property key.separator="|"'
"$ENGINE" exec -d "$C" bash -c 'i=0; while true; do i=$((i+1)); echo "{\"payment_id\":\"pay_$i\",\"status\":\"settled\"}"; sleep 0.3; done | /opt/kafka/bin/kafka-console-producer.sh --bootstrap-server localhost:9092 --topic payments.settled'
"$ENGINE" exec -d "$C" /opt/kafka/bin/kafka-console-consumer.sh --bootstrap-server localhost:9092 --topic orders.created --group fraud-detector
"$ENGINE" exec -d "$C" /opt/kafka/bin/kafka-console-consumer.sh --bootstrap-server localhost:9092 --topic payments.settled --group billing-service
echo "seeded: $(k kafka-topics.sh --list | grep -vc '^__') topics, live traffic on orders.created + payments.settled"
