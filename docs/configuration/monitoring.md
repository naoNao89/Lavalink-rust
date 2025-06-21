---
description: Comprehensive monitoring and metrics setup for Lavalink Rust
---

# Monitoring and Metrics

Lavalink Rust provides comprehensive monitoring capabilities through Prometheus metrics, structured logging, and health endpoints. This guide covers setup, configuration, and best practices for production monitoring.

## Overview

### Monitoring Stack Components

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│  Lavalink Rust  │───▶│   Prometheus    │───▶│     Grafana     │
│                 │    │                 │    │                 │
│ Metrics Export  │    │ Data Collection │    │ Visualization   │
│ Health Checks   │    │ Alerting Rules  │    │ Dashboards      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         ▼                       ▼                       ▼
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│      Logs       │    │  Alertmanager   │    │   Notification  │
│                 │    │                 │    │                 │
│ Structured JSON │    │ Alert Routing   │    │ Slack/Email/etc │
│ Log Aggregation │    │ Deduplication   │    │ Incident Mgmt   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

## Metrics Configuration

### Basic Metrics Setup

```yaml
# application.yml
metrics:
  prometheus:
    enabled: true
    endpoint: "/metrics"
    port: 9090

# Rust-specific metrics
rust:
  metrics:
    # Core metrics
    enabled: true
    collection_interval: 30
    
    # Detailed metrics
    memory_stats: true
    tokio_stats: true
    audio_stats: true
    network_stats: true
    
    # Performance metrics
    track_performance_metrics: true
    player_performance_metrics: true
    fallback_metrics: true
```

### Advanced Metrics Configuration

```yaml
metrics:
  prometheus:
    enabled: true
    endpoint: "/metrics"
    port: 9090
    
    # Security
    auth:
      enabled: true
      username: "metrics"
      password: "${METRICS_PASSWORD}"
    
    # Performance
    cache_size: 10000
    collection_timeout: 30s
    
    # Custom labels
    labels:
      environment: "production"
      datacenter: "us-east-1"
      version: "${LAVALINK_VERSION}"

rust:
  metrics:
    # Collection settings
    enabled: true
    collection_interval: 15
    buffer_size: 1000
    
    # Metric categories
    system_metrics:
      enabled: true
      include_cpu: true
      include_memory: true
      include_disk: true
      include_network: true
    
    runtime_metrics:
      enabled: true
      tokio_tasks: true
      tokio_runtime: true
      thread_pool: true
    
    application_metrics:
      enabled: true
      players: true
      tracks: true
      websockets: true
      http_requests: true
      audio_processing: true
      fallback_system: true
    
    # Custom metrics
    custom_metrics:
      enabled: true
      business_metrics: true
      performance_counters: true
```

## Available Metrics

### Core System Metrics

**Memory Metrics:**
```
# Memory usage in bytes
lavalink_memory_used_bytes{type="heap|non_heap|total"}
lavalink_memory_allocated_bytes
lavalink_memory_deallocated_bytes
lavalink_memory_peak_bytes

# Garbage collection (for cached data)
lavalink_gc_collections_total{type="minor|major"}
lavalink_gc_duration_seconds{type="minor|major"}
```

**CPU Metrics:**
```
# CPU usage percentage
lavalink_cpu_usage_percent{core="0|1|2|..."}
lavalink_cpu_load_average{period="1m|5m|15m"}

# Thread metrics
lavalink_threads_total{state="active|idle|blocked"}
lavalink_thread_pool_size{pool="worker|blocking"}
```

### Audio Processing Metrics

**Player Metrics:**
```
# Active players
lavalink_players_total{state="playing|paused|stopped"}
lavalink_players_by_guild_total

# Player events
lavalink_player_events_total{type="start|end|pause|resume|seek"}
lavalink_player_duration_seconds{type="track|session"}
```

**Track Metrics:**
```
# Track loading
lavalink_tracks_loaded_total{source="youtube|soundcloud|spotify_fallback"}
lavalink_tracks_failed_total{source="youtube|soundcloud|spotify_fallback",reason="not_found|timeout|error"}
lavalink_track_load_duration_seconds{source="youtube|soundcloud|spotify_fallback"}

# Audio processing
lavalink_audio_frames_sent_total
lavalink_audio_frames_lost_total
lavalink_audio_buffer_underruns_total
```

### Network Metrics

**HTTP Metrics:**
```
# HTTP requests
lavalink_http_requests_total{method="GET|POST|PATCH",endpoint="/v4/info|/v4/stats|..."}
lavalink_http_request_duration_seconds{method="GET|POST|PATCH",endpoint="/v4/info|/v4/stats|..."}
lavalink_http_response_size_bytes{method="GET|POST|PATCH",endpoint="/v4/info|/v4/stats|..."}

# HTTP errors
lavalink_http_errors_total{method="GET|POST|PATCH",status="400|401|404|500"}
```

**WebSocket Metrics:**
```
# WebSocket connections
lavalink_websocket_connections_total{state="active|idle"}
lavalink_websocket_messages_total{direction="inbound|outbound",type="text|binary"}
lavalink_websocket_errors_total{type="connection|protocol|timeout"}

# WebSocket performance
lavalink_websocket_message_size_bytes{direction="inbound|outbound"}
lavalink_websocket_latency_seconds
```

### Rust-Specific Metrics

**Tokio Runtime Metrics:**
```
# Task metrics
lavalink_tokio_tasks_total{state="active|idle|completed"}
lavalink_tokio_task_duration_seconds
lavalink_tokio_task_queue_depth

# I/O metrics
lavalink_tokio_io_operations_total{type="read|write"}
lavalink_tokio_io_bytes_total{type="read|write"}
```

**Fallback System Metrics:**
```
# Fallback conversions
lavalink_fallback_conversions_total{service="spotify|apple_music|deezer",result="success|failure"}
lavalink_fallback_search_duration_seconds{service="spotify|apple_music|deezer"}
lavalink_fallback_cache_hits_total{service="spotify|apple_music|deezer"}
lavalink_fallback_cache_misses_total{service="spotify|apple_music|deezer"}
```

## Prometheus Setup

### Prometheus Configuration

```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s
  external_labels:
    cluster: 'lavalink-production'

rule_files:
  - "lavalink_rules.yml"
  - "system_rules.yml"

alerting:
  alertmanagers:
    - static_configs:
        - targets:
          - alertmanager:9093

scrape_configs:
  # Lavalink Rust instances
  - job_name: 'lavalink-rust'
    static_configs:
      - targets: 
        - 'lavalink-1:9090'
        - 'lavalink-2:9090'
        - 'lavalink-3:9090'
    scrape_interval: 10s
    scrape_timeout: 5s
    metrics_path: /metrics
    scheme: http
    
    # Authentication (if enabled)
    basic_auth:
      username: 'metrics'
      password: 'your_metrics_password'
    
    # Relabeling
    relabel_configs:
      - source_labels: [__address__]
        target_label: instance
      - source_labels: [__address__]
        regex: '([^:]+):(.*)'
        target_label: __address__
        replacement: '${1}:9090'

  # System metrics
  - job_name: 'node-exporter'
    static_configs:
      - targets: ['node-exporter:9100']
    scrape_interval: 15s

  # Container metrics
  - job_name: 'cadvisor'
    static_configs:
      - targets: ['cadvisor:8080']
    scrape_interval: 15s
```

### Alerting Rules

```yaml
# lavalink_rules.yml
groups:
  - name: lavalink_alerts
    rules:
      # Service availability
      - alert: LavalinkDown
        expr: up{job="lavalink-rust"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Lavalink instance {{ $labels.instance }} is down"
          description: "Lavalink instance {{ $labels.instance }} has been down for more than 1 minute"

      # High memory usage
      - alert: LavalinkHighMemoryUsage
        expr: (lavalink_memory_used_bytes{type="total"} / 1024 / 1024) > 800
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High memory usage on {{ $labels.instance }}"
          description: "Memory usage is {{ $value }}MB on {{ $labels.instance }}"

      # High CPU usage
      - alert: LavalinkHighCPUUsage
        expr: lavalink_cpu_usage_percent > 80
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High CPU usage on {{ $labels.instance }}"
          description: "CPU usage is {{ $value }}% on {{ $labels.instance }}"

      # Too many players
      - alert: LavalinkTooManyPlayers
        expr: lavalink_players_total > 1000
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "Too many active players on {{ $labels.instance }}"
          description: "{{ $value }} active players on {{ $labels.instance }}"

      # Track loading failures
      - alert: LavalinkTrackLoadingFailures
        expr: rate(lavalink_tracks_failed_total[5m]) > 0.1
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "High track loading failure rate on {{ $labels.instance }}"
          description: "Track loading failure rate is {{ $value }}/sec on {{ $labels.instance }}"

      # WebSocket errors
      - alert: LavalinkWebSocketErrors
        expr: rate(lavalink_websocket_errors_total[5m]) > 0.05
        for: 1m
        labels:
          severity: warning
        annotations:
          summary: "WebSocket errors on {{ $labels.instance }}"
          description: "WebSocket error rate is {{ $value }}/sec on {{ $labels.instance }}"

      # Fallback system issues
      - alert: LavalinkFallbackFailures
        expr: rate(lavalink_fallback_conversions_total{result="failure"}[5m]) > 0.2
        for: 3m
        labels:
          severity: warning
        annotations:
          summary: "High fallback failure rate on {{ $labels.instance }}"
          description: "Fallback failure rate is {{ $value }}/sec on {{ $labels.instance }}"
```

## Grafana Dashboards

### Main Dashboard Configuration

```json
{
  "dashboard": {
    "id": null,
    "title": "Lavalink Rust Monitoring",
    "tags": ["lavalink", "rust", "audio"],
    "timezone": "browser",
    "panels": [
      {
        "id": 1,
        "title": "Service Overview",
        "type": "stat",
        "targets": [
          {
            "expr": "up{job=\"lavalink-rust\"}",
            "legendFormat": "{{instance}} Status"
          }
        ],
        "fieldConfig": {
          "defaults": {
            "color": {
              "mode": "thresholds"
            },
            "thresholds": {
              "steps": [
                {"color": "red", "value": 0},
                {"color": "green", "value": 1}
              ]
            }
          }
        }
      },
      {
        "id": 2,
        "title": "Memory Usage",
        "type": "timeseries",
        "targets": [
          {
            "expr": "lavalink_memory_used_bytes{type=\"total\"} / 1024 / 1024",
            "legendFormat": "{{instance}} Memory (MB)"
          }
        ]
      },
      {
        "id": 3,
        "title": "Active Players",
        "type": "timeseries",
        "targets": [
          {
            "expr": "lavalink_players_total{state=\"playing\"}",
            "legendFormat": "{{instance}} Playing"
          },
          {
            "expr": "lavalink_players_total",
            "legendFormat": "{{instance}} Total"
          }
        ]
      },
      {
        "id": 4,
        "title": "Track Loading Rate",
        "type": "timeseries",
        "targets": [
          {
            "expr": "rate(lavalink_tracks_loaded_total[5m])",
            "legendFormat": "{{instance}} {{source}} Success"
          },
          {
            "expr": "rate(lavalink_tracks_failed_total[5m])",
            "legendFormat": "{{instance}} {{source}} Failed"
          }
        ]
      },
      {
        "id": 5,
        "title": "HTTP Request Rate",
        "type": "timeseries",
        "targets": [
          {
            "expr": "rate(lavalink_http_requests_total[5m])",
            "legendFormat": "{{instance}} {{method}} {{endpoint}}"
          }
        ]
      },
      {
        "id": 6,
        "title": "WebSocket Connections",
        "type": "timeseries",
        "targets": [
          {
            "expr": "lavalink_websocket_connections_total{state=\"active\"}",
            "legendFormat": "{{instance}} Active Connections"
          }
        ]
      }
    ]
  }
}
```

### Performance Dashboard

```json
{
  "dashboard": {
    "title": "Lavalink Rust Performance",
    "panels": [
      {
        "title": "CPU Usage",
        "type": "timeseries",
        "targets": [
          {
            "expr": "lavalink_cpu_usage_percent",
            "legendFormat": "{{instance}} CPU %"
          }
        ]
      },
      {
        "title": "Tokio Tasks",
        "type": "timeseries",
        "targets": [
          {
            "expr": "lavalink_tokio_tasks_total{state=\"active\"}",
            "legendFormat": "{{instance}} Active Tasks"
          }
        ]
      },
      {
        "title": "Audio Processing",
        "type": "timeseries",
        "targets": [
          {
            "expr": "rate(lavalink_audio_frames_sent_total[5m])",
            "legendFormat": "{{instance}} Frames/sec"
          }
        ]
      }
    ]
  }
}
```

## Logging Configuration

### Structured Logging Setup

```yaml
# application.yml
logging:
  level:
    root: INFO
    lavalink: INFO
    rust: INFO
  
  # File logging
  file:
    enabled: true
    path: "./logs/lavalink.log"
    max_size: "100MB"
    max_files: 10
    
  # JSON format for log aggregation
  format: json
  
  # Custom loggers
  loggers:
    "lavalink.audio": DEBUG
    "lavalink.websocket": INFO
    "lavalink.fallback": INFO

# Rust-specific logging
rust:
  logging:
    # Structured logging
    format: "json"
    timestamp_format: "rfc3339"
    
    # Log levels by module
    levels:
      "lavalink_rust": "info"
      "lavalink_rust::audio": "debug"
      "lavalink_rust::websocket": "info"
      "lavalink_rust::fallback": "info"
    
    # Performance logging
    performance_logs: true
    slow_query_threshold: "1s"
    
    # Security logging
    security_events: true
    failed_auth_attempts: true
```

### Log Aggregation with Loki

```yaml
# loki.yml
auth_enabled: false

server:
  http_listen_port: 3100

ingester:
  lifecycler:
    address: 127.0.0.1
    ring:
      kvstore:
        store: inmemory
      replication_factor: 1
    final_sleep: 0s

schema_config:
  configs:
    - from: 2020-10-24
      store: boltdb
      object_store: filesystem
      schema: v11
      index:
        prefix: index_
        period: 168h

storage_config:
  boltdb:
    directory: /loki/index
  filesystem:
    directory: /loki/chunks

limits_config:
  enforce_metric_name: false
  reject_old_samples: true
  reject_old_samples_max_age: 168h
```

## Health Checks

### Built-in Health Endpoints

```bash
# Basic server info
curl http://localhost:2333/v4/info

# Server statistics
curl http://localhost:2333/v4/stats

# Health check endpoint
curl http://localhost:2333/v4/health

# Readiness probe
curl http://localhost:2333/v4/ready

# Liveness probe
curl http://localhost:2333/v4/alive
```

### Custom Health Checks

```yaml
# application.yml
health:
  enabled: true
  endpoint: "/health"
  
  # Health check components
  checks:
    database: true
    audio_sources: true
    websocket: true
    memory: true
    disk_space: true
    
  # Thresholds
  thresholds:
    memory_usage_percent: 90
    disk_usage_percent: 85
    response_time_ms: 1000
```

## Alerting Setup

### Alertmanager Configuration

```yaml
# alertmanager.yml
global:
  smtp_smarthost: 'localhost:587'
  smtp_from: 'alerts@example.com'

route:
  group_by: ['alertname']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 1h
  receiver: 'web.hook'
  routes:
    - match:
        severity: critical
      receiver: 'critical-alerts'
    - match:
        severity: warning
      receiver: 'warning-alerts'

receivers:
  - name: 'web.hook'
    webhook_configs:
      - url: 'http://127.0.0.1:5001/'

  - name: 'critical-alerts'
    email_configs:
      - to: 'oncall@example.com'
        subject: 'CRITICAL: Lavalink Alert'
        body: |
          {{ range .Alerts }}
          Alert: {{ .Annotations.summary }}
          Description: {{ .Annotations.description }}
          {{ end }}
    slack_configs:
      - api_url: 'YOUR_SLACK_WEBHOOK_URL'
        channel: '#alerts'
        title: 'CRITICAL: Lavalink Alert'

  - name: 'warning-alerts'
    email_configs:
      - to: 'team@example.com'
        subject: 'WARNING: Lavalink Alert'
```

## Best Practices

### Monitoring Strategy

1. **Layered Monitoring**: Implement monitoring at multiple levels (infrastructure, application, business)
2. **Proactive Alerting**: Set up alerts before issues become critical
3. **Baseline Establishment**: Establish performance baselines for comparison
4. **Regular Review**: Regularly review and update monitoring configurations

### Performance Optimization

1. **Metric Selection**: Only collect metrics that provide actionable insights
2. **Retention Policies**: Set appropriate data retention policies
3. **Sampling**: Use sampling for high-cardinality metrics
4. **Aggregation**: Pre-aggregate metrics where possible

### Security Considerations

1. **Access Control**: Secure access to monitoring endpoints
2. **Data Privacy**: Ensure sensitive data is not exposed in metrics
3. **Network Security**: Use secure communication channels
4. **Audit Logging**: Log access to monitoring systems

For more information, see:
- [Performance Tuning](../advanced/performance.md)
- [Docker Deployment](../advanced/docker-deployment.md)
- [Security Configuration](security.md)
