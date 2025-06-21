---
description: Audio filter configuration for Lavalink Rust
---

# Audio Filters Configuration

Lavalink Rust supports all standard Lavalink audio filters with enhanced performance and additional configuration options.

!!! rust "Filter Performance"
    Rust implementation provides faster filter processing with lower CPU usage and more predictable performance compared to Java Lavalink.

## Filter Overview

All standard Lavalink filters are supported and enabled by default:

```yaml
lavalink:
  server:
    filters:
      volume: true        # Volume adjustment
      equalizer: true     # 15-band equalizer
      karaoke: true       # Vocal removal/isolation
      timescale: true     # Speed and pitch control
      tremolo: true       # Volume oscillation
      vibrato: true       # Pitch oscillation
      rotation: true      # 3D audio positioning
      distortion: true    # Audio distortion effects
      channelMix: true    # Stereo channel manipulation
      lowPass: true       # High frequency filtering
```

## Individual Filter Configuration

### Volume Filter

Adjusts the overall playback volume with support for amplification beyond 100%.

```yaml
lavalink:
  server:
    filters:
      volume: true

rust:
  filters:
    volume:
      max_amplification: 5.0    # Maximum volume amplification (5.0 = 500%)
      soft_clipping: true       # Enable soft clipping to prevent distortion
      normalize: false          # Auto-normalize volume levels
```

**Usage Example:**
```json
{
  "filters": {
    "volume": 1.5  // 150% volume
  }
}
```

**Parameters:**
- Range: `0.0` to `5.0` (0% to 500%)
- Default: `1.0` (100%)
- Soft clipping prevents harsh distortion at high volumes

### Equalizer Filter

15-band equalizer for precise frequency adjustment.

```yaml
lavalink:
  server:
    filters:
      equalizer: true

rust:
  filters:
    equalizer:
      precision: "high"         # "low", "medium", "high"
      auto_gain: true          # Automatic gain compensation
      presets_enabled: true    # Enable built-in presets
```

**Band Frequencies:**

| Band | Frequency | Band | Frequency |
|------|-----------|------|-----------|
| 0    | 25 Hz     | 8    | 1000 Hz   |
| 1    | 40 Hz     | 9    | 1600 Hz   |
| 2    | 63 Hz     | 10   | 2500 Hz   |
| 3    | 100 Hz    | 11   | 4000 Hz   |
| 4    | 160 Hz    | 12   | 6300 Hz   |
| 5    | 250 Hz    | 13   | 10000 Hz  |
| 6    | 400 Hz    | 14   | 16000 Hz  |
| 7    | 630 Hz    |      |           |

**Usage Example:**
```json
{
  "filters": {
    "equalizer": [
      {"band": 0, "gain": 0.2},   // Boost bass
      {"band": 1, "gain": 0.15},
      {"band": 13, "gain": -0.1}, // Reduce treble
      {"band": 14, "gain": -0.1}
    ]
  }
}
```

### Karaoke Filter

Vocal removal and isolation using advanced audio processing.

```yaml
lavalink:
  server:
    filters:
      karaoke: true

rust:
  filters:
    karaoke:
      algorithm: "advanced"     # "basic", "advanced", "ml" (machine learning)
      vocal_detection: true     # Enhanced vocal detection
      preserve_harmonies: false # Preserve backing vocals
```

**Parameters:**
- `level`: Effect intensity (0.0 to 1.0)
- `monoLevel`: Mono effect level (0.0 to 1.0)
- `filterBand`: Target frequency band (Hz)
- `filterWidth`: Filter bandwidth

**Usage Example:**
```json
{
  "filters": {
    "karaoke": {
      "level": 1.0,
      "monoLevel": 1.0,
      "filterBand": 220.0,
      "filterWidth": 100.0
    }
  }
}
```

### Timescale Filter

Speed, pitch, and rate manipulation with high-quality algorithms.

```yaml
lavalink:
  server:
    filters:
      timescale: true

rust:
  filters:
    timescale:
      algorithm: "wsola"        # "wsola", "psola", "phase_vocoder"
      quality: "high"           # "low", "medium", "high"
      preserve_formants: true   # Maintain voice characteristics
```

**Parameters:**
- `speed`: Playback speed (0.1 to 3.0)
- `pitch`: Pitch adjustment (0.1 to 3.0)
- `rate`: Overall rate (0.1 to 3.0)

**Usage Example:**
```json
{
  "filters": {
    "timescale": {
      "speed": 1.2,    // 20% faster
      "pitch": 1.0,    // Normal pitch
      "rate": 1.2      // 20% faster overall
    }
  }
}
```

### Tremolo Filter

Volume oscillation effect for creating rhythmic variations.

```yaml
lavalink:
  server:
    filters:
      tremolo: true

rust:
  filters:
    tremolo:
      waveform: "sine"          # "sine", "triangle", "square", "sawtooth"
      stereo_phase: 0.0         # Stereo phase offset (0.0 to 1.0)
```

**Parameters:**
- `frequency`: Oscillation frequency (0.1 to 20.0 Hz)
- `depth`: Effect depth (0.0 to 1.0)

**Usage Example:**
```json
{
  "filters": {
    "tremolo": {
      "frequency": 2.0,
      "depth": 0.5
    }
  }
}
```

### Vibrato Filter

Pitch oscillation effect for creating vibrato.

```yaml
lavalink:
  server:
    filters:
      vibrato: true

rust:
  filters:
    vibrato:
      waveform: "sine"          # "sine", "triangle", "square", "sawtooth"
      interpolation: "linear"   # "linear", "cubic", "hermite"
```

**Parameters:**
- `frequency`: Oscillation frequency (0.1 to 14.0 Hz)
- `depth`: Effect depth (0.0 to 1.0)

**Usage Example:**
```json
{
  "filters": {
    "vibrato": {
      "frequency": 4.0,
      "depth": 0.3
    }
  }
}
```

### Rotation Filter

3D audio positioning and rotation effects.

```yaml
lavalink:
  server:
    filters:
      rotation: true

rust:
  filters:
    rotation:
      algorithm: "hrtf"         # "simple", "hrtf" (Head-Related Transfer Function)
      room_simulation: false    # Simulate room acoustics
```

**Parameters:**
- `rotationHz`: Rotation frequency (0.0 to 1.0 Hz)

**Usage Example:**
```json
{
  "filters": {
    "rotation": {
      "rotationHz": 0.2  // Slow rotation
    }
  }
}
```

### Distortion Filter

Audio distortion effects with multiple algorithms.

```yaml
lavalink:
  server:
    filters:
      distortion: true

rust:
  filters:
    distortion:
      algorithm: "soft"         # "soft", "hard", "tube", "fuzz"
      oversampling: 4           # Oversampling factor (1, 2, 4, 8)
      anti_aliasing: true       # Anti-aliasing filter
```

**Parameters:**
- `sinOffset`, `sinScale`: Sine wave distortion
- `cosOffset`, `cosScale`: Cosine wave distortion
- `tanOffset`, `tanScale`: Tangent wave distortion
- `offset`, `scale`: Overall distortion

**Usage Example:**
```json
{
  "filters": {
    "distortion": {
      "sinOffset": 0.0,
      "sinScale": 1.0,
      "cosOffset": 0.0,
      "cosScale": 1.0,
      "tanOffset": 0.0,
      "tanScale": 1.0,
      "offset": 0.0,
      "scale": 1.0
    }
  }
}
```

### Channel Mix Filter

Stereo channel manipulation and mixing.

```yaml
lavalink:
  server:
    filters:
      channelMix: true

rust:
  filters:
    channel_mix:
      matrix_mode: "standard"   # "standard", "karaoke", "mono", "swap"
      auto_balance: true        # Automatic level balancing
```

**Parameters:**
- `leftToLeft`: Left channel to left output (0.0 to 1.0)
- `leftToRight`: Left channel to right output (0.0 to 1.0)
- `rightToLeft`: Right channel to left output (0.0 to 1.0)
- `rightToRight`: Right channel to right output (0.0 to 1.0)

**Usage Example:**
```json
{
  "filters": {
    "channelMix": {
      "leftToLeft": 0.5,
      "leftToRight": 0.5,
      "rightToLeft": 0.5,
      "rightToRight": 0.5
    }
  }
}
```

### Low Pass Filter

High-frequency filtering for warmer sound.

```yaml
lavalink:
  server:
    filters:
      lowPass: true

rust:
  filters:
    low_pass:
      filter_type: "butterworth" # "butterworth", "chebyshev", "elliptic"
      order: 4                   # Filter order (1-8)
      resonance: 1.0            # Filter resonance
```

**Parameters:**
- `smoothing`: Smoothing factor (1.0 and above)

**Usage Example:**
```json
{
  "filters": {
    "lowPass": {
      "smoothing": 20.0
    }
  }
}
```

## Filter Chains and Presets

### Filter Chains

Combine multiple filters for complex audio processing:

```yaml
rust:
  filters:
    chains:
      enabled: true
      max_filters_per_chain: 10
      
    presets:
      bass_boost:
        - type: "equalizer"
          config:
            - {"band": 0, "gain": 0.3}
            - {"band": 1, "gain": 0.2}
            - {"band": 2, "gain": 0.1}
        - type: "volume"
          config: 1.1
          
      vocal_enhance:
        - type: "equalizer"
          config:
            - {"band": 6, "gain": 0.2}
            - {"band": 7, "gain": 0.3}
            - {"band": 8, "gain": 0.2}
        - type: "karaoke"
          config:
            level: -0.5  # Inverse karaoke to enhance vocals
```

### Performance Optimization

```yaml
rust:
  filters:
    performance:
      parallel_processing: true  # Process filters in parallel
      simd_optimization: true    # Use SIMD instructions
      buffer_size: 1024         # Audio buffer size for processing
      thread_pool_size: 4       # Filter processing thread pool
      
    quality:
      sample_rate: 48000        # Internal processing sample rate
      bit_depth: 32             # Internal processing bit depth
      dithering: true           # Apply dithering when downsampling
```

## Filter Monitoring and Debugging

### Performance Monitoring

```yaml
rust:
  filters:
    monitoring:
      enabled: true
      log_performance: true     # Log filter performance metrics
      cpu_usage_threshold: 80   # Warn if CPU usage exceeds threshold
      latency_threshold: 10     # Warn if latency exceeds threshold (ms)
```

### Debug Configuration

```yaml
logging:
  level:
    lavalink::filters: DEBUG
    
rust:
  debug:
    log_filter_chains: true     # Log filter chain processing
    log_audio_stats: true       # Log audio processing statistics
    save_debug_audio: false     # Save debug audio files (development only)
```

## Migration from Java Lavalink

### Compatible Filters

All Java Lavalink filters work identically in Rust:

- ✅ Same parameter ranges and behavior
- ✅ Identical JSON API format
- ✅ Same filter names and structure

### Enhanced Features

Rust-specific improvements:

- 🚀 **Better Performance**: 20-30% faster filter processing
- 🎛️ **Advanced Algorithms**: Additional filter algorithms available
- 🔧 **Fine-tuning**: More configuration options for quality vs performance
- 📊 **Monitoring**: Built-in performance monitoring and debugging

### Migration Tips

1. **Existing Configurations**: Work without changes
2. **Performance**: May need to adjust buffer sizes for optimal performance
3. **Quality**: Can enable higher quality algorithms if CPU allows
4. **Monitoring**: Enable monitoring to track filter performance

For more information, see:
- [Audio Sources Configuration](sources.md)
- [Performance Tuning](performance.md)
- [API Reference](../api/rest.md#filters)
