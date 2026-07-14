import SwiftUI

// MARK: - Twin Hardware View

struct TwinHardwareView: View {
    @EnvironmentObject var runner: XMacRunner

    var body: some View {
        twinDetailContainer("Hardware Profile", icon: "cpu.fill") {
            if let twin = runner.digitalTwin {
                hardwareContent(twin.hardware)
            } else {
                twinNoDataView
            }
        }
    }

    private func hardwareContent(_ hw: HardwareProfile) -> some View {
        VStack(spacing: 16) {
            // Machine identity
            XCard {
                VStack(alignment: .leading, spacing: 12) {
                    XSectionHeader(title: "Machine Identity", icon: "chip.fill")
                    twinRow("Model", hw.model_identifier)
                    twinRow("SoC Generation", hw.soc_generation)
                    twinRow("Fingerprint", String(hw.fingerprint.prefix(16)) + "...")
                }
            }

            // CPU
            XCard {
                VStack(alignment: .leading, spacing: 12) {
                    XSectionHeader(title: "CPU Topology", icon: "cpu")
                    twinRow("Total Logical Cores", "\(hw.cpu_cores.total_logical_cores)")
                    twinRow("Performance Cores", "\(hw.cpu_cores.performance_cores)")
                    twinRow("Efficiency Cores", "\(hw.cpu_cores.efficiency_cores)")
                }
            }

            HStack(spacing: 16) {
                // GPU
                XCard {
                    VStack(alignment: .leading, spacing: 12) {
                        XSectionHeader(title: "GPU", icon: "display")
                        twinRow("Core Count", "\(hw.gpu.core_count)")
                        if let util = hw.gpu.utilization_pct {
                            twinRow("Utilization", String(format: "%.1f%%", util))
                        }
                    }
                }
                // Neural Engine
                XCard {
                    VStack(alignment: .leading, spacing: 12) {
                        XSectionHeader(title: "Neural Engine", icon: "brain.head.profile")
                        twinRow("Core Count", "\(hw.neural_engine.core_count)")
                        if let util = hw.neural_engine.utilization_pct {
                            twinRow("Utilization", String(format: "%.1f%%", util))
                        }
                    }
                }
            }

            // Memory
            XCard {
                VStack(alignment: .leading, spacing: 12) {
                    XSectionHeader(title: "Memory", icon: "memorychip")
                    twinRow("Total", formatBytes(hw.memory.total_bytes))
                    if let bw = hw.memory.bandwidth_gbps {
                        twinRow("Bandwidth", String(format: "%.0f GB/s", bw))
                    }
                }
            }

            // Storage
            if !hw.storage.isEmpty {
                XCard {
                    VStack(alignment: .leading, spacing: 12) {
                        XSectionHeader(title: "Storage", icon: "internaldrive", count: hw.storage.count)
                        ForEach(hw.storage.indices, id: \.self) { i in
                            let dev = hw.storage[i]
                            HStack(spacing: 8) {
                                Image(systemName: dev.is_ssd ? "internaldrive.fill" : "internaldrive")
                                    .foregroundStyle(XTheme.accent)
                                VStack(alignment: .leading, spacing: 2) {
                                    Text(dev.name)
                                        .font(.system(size: 13, weight: .medium))
                                        .foregroundStyle(XTheme.textPrimary)
                                    Text("\(formatBytes(dev.capacity_bytes)) \(dev.is_ssd ? "SSD" : "HDD")")
                                        .font(.system(size: 11))
                                        .foregroundStyle(XTheme.textSecondary)
                                }
                                Spacer()
                                if let smart = dev.smart_health {
                                    Text(smart.status)
                                        .font(.system(size: 11, weight: .medium))
                                        .foregroundStyle(smart.status.lowercased() == "ok" ? XTheme.safe : XTheme.medium)
                                }
                            }
                        }
                    }
                }
            }

            // Battery
            if let bat = hw.battery {
                XCard {
                    VStack(alignment: .leading, spacing: 12) {
                        XSectionHeader(title: "Battery", icon: "battery.100")
                        twinRow("Model", bat.model)
                        twinRow("Chemistry", bat.chemistry)
                        twinRow("Cycle Count", "\(bat.cycle_count)")
                        if let cap = bat.design_capacity_mah {
                            twinRow("Design Capacity", "\(cap) mAh")
                        }
                    }
                }
            }

            // Thermal
            XCard {
                VStack(alignment: .leading, spacing: 12) {
                    XSectionHeader(title: "Thermal", icon: "thermometer")
                    if let temp = hw.thermal.cpu_temp_c {
                        twinRow("CPU Temp", String(format: "%.1f°C", temp))
                    }
                    if let temp = hw.thermal.gpu_temp_c {
                        twinRow("GPU Temp", String(format: "%.1f°C", temp))
                    }
                    twinRow("Thermal Pressure", hw.thermal.thermal_pressure)
                    if !hw.thermal.fan_speeds_rpm.isEmpty {
                        twinRow("Fan Speeds", hw.thermal.fan_speeds_rpm.map { "\($0) RPM" }.joined(separator: ", "))
                    }
                }
            }

            // Power State
            XCard {
                VStack(alignment: .leading, spacing: 12) {
                    XSectionHeader(title: "Power State", icon: "power")
                    twinRow("On Battery", hw.power_state.is_on_battery ? "Yes" : "No")
                    twinRow("Charging", hw.power_state.is_charging ? "Yes" : "No")
                    twinRow("Sleep State", hw.power_state.sleep_state)
                    twinRow("Low Power Mode", hw.power_state.low_power_mode ? "Yes" : "No")
                }
            }

            // Peripherals
            XCard {
                VStack(alignment: .leading, spacing: 12) {
                    XSectionHeader(title: "Peripherals", icon: "cable.connector")
                    if !hw.peripherals.usb_devices.isEmpty {
                        twinRow("USB", hw.peripherals.usb_devices.joined(separator: ", "))
                    }
                    if !hw.peripherals.external_displays.isEmpty {
                        twinRow("Displays", hw.peripherals.external_displays.joined(separator: ", "))
                    }
                    if !hw.peripherals.bluetooth_devices.isEmpty {
                        twinRow("Bluetooth", hw.peripherals.bluetooth_devices.joined(separator: ", "))
                    }
                    if !hw.peripherals.network_interfaces.isEmpty {
                        twinRow("Network", hw.peripherals.network_interfaces.joined(separator: ", "))
                    }
                }
            }
        }
    }
}

// MARK: - Twin Software View

struct TwinSoftwareView: View {
    @EnvironmentObject var runner: XMacRunner

    var body: some View {
        twinDetailContainer("Software Genome", icon: "shippingbox.fill") {
            if let twin = runner.digitalTwin {
                softwareContent(twin.software_genome)
            } else {
                twinNoDataView
            }
        }
    }

    private func softwareContent(_ sg: SoftwareGenome) -> some View {
        VStack(spacing: 16) {
            XCard {
                VStack(alignment: .leading, spacing: 8) {
                    XSectionHeader(title: "Total Components", icon: "number")
                    Text("\(sg.total_components)")
                        .font(.system(size: 32, weight: .bold, design: .rounded))
                        .foregroundStyle(XTheme.accent)
                }
            }

            // Applications
            if !sg.applications.isEmpty {
                XCard {
                    VStack(alignment: .leading, spacing: 8) {
                        XSectionHeader(title: "Applications", icon: "app.fill", count: sg.applications.count)
                        ForEach(sg.applications.sorted { $0.size_bytes > $1.size_bytes }.prefix(20)) { app in
                            HStack(spacing: 8) {
                                Image(systemName: "app")
                                    .foregroundStyle(XTheme.accent)
                                    .font(.system(size: 12))
                                VStack(alignment: .leading, spacing: 1) {
                                    Text(app.name)
                                        .font(.system(size: 12, weight: .medium))
                                        .foregroundStyle(XTheme.textPrimary)
                                    Text(app.version)
                                        .font(.system(size: 10, design: .monospaced))
                                        .foregroundStyle(XTheme.textTertiary)
                                }
                                Spacer()
                                Text(app.sizeFormatted)
                                    .font(.system(size: 11, design: .monospaced))
                                    .foregroundStyle(XTheme.textSecondary)
                            }
                        }
                    }
                }
            }

            // Component categories grid
            LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible()), GridItem(.flexible())], spacing: 12) {
                componentCountCard("Frameworks", sg.frameworks.count, "shippingbox")
                componentCountCard("Dylibs", sg.dynamic_libraries.count, "doc.text.fill")
                componentCountCard("Kexts", sg.kernel_extensions.count, "gearshape.2.fill")
                componentCountCard("System Ext", sg.system_extensions.count, "shield.lefthalf.filled")
                componentCountCard("Launch Agents", sg.launch_agents.count, "rocket.fill")
                componentCountCard("Launch Daemons", sg.launch_daemons.count, "terminal.fill")
                componentCountCard("Login Items", sg.login_items.count, "person.fill.checkmark")
                componentCountCard("Plugins", sg.plugins.count, "puzzlepiece.fill")
                componentCountCard("Browser Ext", sg.browser_extensions.count, "safari.fill")
                componentCountCard("Fonts", sg.fonts.count, "textformat")
                componentCountCard("Dev Tools", sg.developer_tools.count, "hammer.fill")
                componentCountCard("SDKs", sg.sdks.count, "wrench.and.screwdriver.fill")
                componentCountCard("Pkg Managers", sg.package_managers.count, "shippingbox.fill")
                componentCountCard("Python Envs", sg.python_envs.count, "python")
                componentCountCard("Node Envs", sg.node_envs.count, "network")
                componentCountCard("Rust Toolchains", sg.rust_toolchains.count, "rust")
                componentCountCard("Docker Images", sg.docker_images.count, "cylinder.fill")
                componentCountCard("Containers", sg.containers.count, "cylinder.split.1x2.fill")
                componentCountCard("VMs", sg.virtual_machines.count, "display")
                componentCountCard("AI Models", sg.ai_models.count, "brain")
                componentCountCard("Datasets", sg.datasets.count, "tablecells.fill")
            }
        }
    }

    private func componentCountCard(_ title: String, _ count: Int, _ icon: String) -> some View {
        XCard {
            VStack(spacing: 6) {
                Image(systemName: icon)
                    .font(.system(size: 18))
                    .foregroundStyle(XTheme.accent)
                Text("\(count)")
                    .font(.system(size: 20, weight: .bold, design: .rounded))
                    .foregroundStyle(XTheme.textPrimary)
                Text(title)
                    .font(.system(size: 10))
                    .foregroundStyle(XTheme.textSecondary)
            }
            .frame(maxWidth: .infinity)
        }
    }
}

// MARK: - Twin Filesystem View

struct TwinFilesystemView: View {
    @EnvironmentObject var runner: XMacRunner

    var body: some View {
        twinDetailContainer("Filesystem Graph", icon: "folder.fill.badge.gearshape") {
            if let twin = runner.digitalTwin {
                fsContent(twin.filesystem)
            } else {
                twinNoDataView
            }
        }
    }

    private func fsContent(_ fs: FilesystemGraph) -> some View {
        VStack(spacing: 16) {
            HStack(spacing: 16) {
                XCard {
                    VStack(spacing: 6) {
                        Text(formatBytes(fs.total_size_bytes))
                            .font(.system(size: 24, weight: .bold, design: .rounded))
                            .foregroundStyle(XTheme.accent)
                        Text("Total Size")
                            .font(.system(size: 11))
                            .foregroundStyle(XTheme.textSecondary)
                    }
                    .frame(maxWidth: .infinity)
                }
                XCard {
                    VStack(spacing: 6) {
                        Text("\(fs.total_files)")
                            .font(.system(size: 24, weight: .bold, design: .rounded))
                            .foregroundStyle(XTheme.textPrimary)
                        Text("Total Files")
                            .font(.system(size: 11))
                            .foregroundStyle(XTheme.textSecondary)
                    }
                    .frame(maxWidth: .infinity)
                }
                if let trend = fs.storage_growth_trend {
                    XCard {
                        VStack(spacing: 6) {
                            Text(String(format: "%.1f%%", trend * 100))
                                .font(.system(size: 24, weight: .bold, design: .rounded))
                                .foregroundStyle(trend > 0 ? XTheme.medium : XTheme.safe)
                            Text("Growth Trend")
                                .font(.system(size: 11))
                                .foregroundStyle(XTheme.textSecondary)
                        }
                        .frame(maxWidth: .infinity)
                    }
                }
                if let forecast = fs.exhaustion_forecast_days {
                    XCard {
                        VStack(spacing: 6) {
                            Text("\(forecast)")
                                .font(.system(size: 24, weight: .bold, design: .rounded))
                                .foregroundStyle(forecast < 30 ? XTheme.critical : XTheme.textPrimary)
                            Text("Days to Full")
                                .font(.system(size: 11))
                                .foregroundStyle(XTheme.textSecondary)
                        }
                        .frame(maxWidth: .infinity)
                    }
                }
            }

            // Duplicate clusters
            if !fs.duplicate_clusters.isEmpty {
                XCard {
                    VStack(alignment: .leading, spacing: 8) {
                        XSectionHeader(title: "Duplicate Clusters", icon: "doc.on.doc.fill", count: fs.duplicate_clusters.count)
                        ForEach(fs.duplicate_clusters.sorted { $0.total_size_bytes > $1.total_size_bytes }.prefix(20)) { cluster in
                            HStack(spacing: 8) {
                                Image(systemName: "doc.on.doc.fill")
                                    .foregroundStyle(XTheme.medium)
                                    .font(.system(size: 12))
                                VStack(alignment: .leading, spacing: 1) {
                                    Text("\(cluster.fileCount) files")
                                        .font(.system(size: 12, weight: .medium))
                                        .foregroundStyle(XTheme.textPrimary)
                                    Text(String(cluster.hash.prefix(16)))
                                        .font(.system(size: 10, design: .monospaced))
                                        .foregroundStyle(XTheme.textTertiary)
                                }
                                Spacer()
                                Text(cluster.sizeFormatted)
                                    .font(.system(size: 11, design: .monospaced))
                                    .foregroundStyle(XTheme.medium)
                            }
                        }
                    }
                }
            }

            // Abandoned & orphan files
            if !fs.abandoned_files.isEmpty || !fs.orphan_files.isEmpty {
                XCard {
                    VStack(alignment: .leading, spacing: 8) {
                        if !fs.abandoned_files.isEmpty {
                            XSectionHeader(title: "Abandoned Files", icon: "trash.fill", count: fs.abandoned_files.count)
                            ForEach(fs.abandoned_files.prefix(10), id: \.self) { path in
                                Text(path)
                                    .font(.system(size: 11, design: .monospaced))
                                    .foregroundStyle(XTheme.textSecondary)
                                    .lineLimit(1)
                                    .truncationMode(.middle)
                            }
                        }
                        if !fs.orphan_files.isEmpty {
                            XSectionHeader(title: "Orphan Files", icon: "questionmark.folder.fill", count: fs.orphan_files.count)
                            ForEach(fs.orphan_files.prefix(10), id: \.self) { path in
                                Text(path)
                                    .font(.system(size: 11, design: .monospaced))
                                    .foregroundStyle(XTheme.textSecondary)
                                    .lineLimit(1)
                                    .truncationMode(.middle)
                            }
                        }
                    }
                }
            }
        }
    }
}

// MARK: - Twin Process View

struct TwinProcessView: View {
    @EnvironmentObject var runner: XMacRunner

    var body: some View {
        twinDetailContainer("Process Intelligence", icon: "chart.bar.fill") {
            if let twin = runner.digitalTwin {
                processContent(twin.processes)
            } else {
                twinNoDataView
            }
        }
    }

    private func processContent(_ proc: ProcessIntelligenceGraph) -> some View {
        VStack(spacing: 16) {
            XCard {
                VStack(spacing: 6) {
                    Text("\(proc.total_processes)")
                        .font(.system(size: 28, weight: .bold, design: .rounded))
                        .foregroundStyle(XTheme.accent)
                    Text("Total Processes")
                        .font(.system(size: 11))
                        .foregroundStyle(XTheme.textSecondary)
                }
                .frame(maxWidth: .infinity)
            }

            // Top CPU consumers
            XCard {
                VStack(alignment: .leading, spacing: 8) {
                    XSectionHeader(title: "Top CPU Consumers", icon: "flame.fill")
                    ForEach(proc.process_tree.sorted { $0.cpu_pct > $1.cpu_pct }.prefix(20)) { p in
                        HStack(spacing: 8) {
                            Image(systemName: "flame.fill")
                                .foregroundStyle(p.cpu_pct > 50 ? XTheme.critical : p.cpu_pct > 20 ? XTheme.high : XTheme.medium)
                                .font(.system(size: 11))
                            Text(p.name)
                                .font(.system(size: 12, weight: .medium))
                                .foregroundStyle(XTheme.textPrimary)
                                .lineLimit(1)
                            Text("PID \(p.pid)")
                                .font(.system(size: 10, design: .monospaced))
                                .foregroundStyle(XTheme.textTertiary)
                            Spacer()
                            Text(String(format: "%.1f%%", p.cpu_pct))
                                .font(.system(size: 11, design: .monospaced))
                                .foregroundStyle(XTheme.textSecondary)
                            Text(p.memoryFormatted)
                                .font(.system(size: 11, design: .monospaced))
                                .foregroundStyle(XTheme.textTertiary)
                        }
                    }
                }
            }

            // Anomalies
            if !proc.anomalies.isEmpty {
                XCard {
                    VStack(alignment: .leading, spacing: 8) {
                        XSectionHeader(title: "Anomalies", icon: "exclamationmark.triangle.fill", count: proc.anomalies.count)
                        ForEach(proc.anomalies) { a in
                            HStack(spacing: 8) {
                                Image(systemName: "exclamationmark.triangle.fill")
                                    .foregroundStyle(severityColor(a.severity))
                                    .font(.system(size: 11))
                                VStack(alignment: .leading, spacing: 1) {
                                    Text("\(a.name) (PID \(a.pid))")
                                        .font(.system(size: 12, weight: .medium))
                                        .foregroundStyle(XTheme.textPrimary)
                                    Text(a.description)
                                        .font(.system(size: 11))
                                        .foregroundStyle(XTheme.textSecondary)
                                        .lineLimit(2)
                                }
                                Spacer()
                                Text(a.anomaly_type)
                                    .font(.system(size: 10, design: .monospaced))
                                    .foregroundStyle(severityColor(a.severity))
                            }
                        }
                    }
                }
            }

            // Bottleneck predictions
            if !proc.bottleneck_predictions.isEmpty {
                XCard {
                    VStack(alignment: .leading, spacing: 8) {
                        XSectionHeader(title: "Bottleneck Predictions", icon: "chart.line.downtrend.xyaxis", count: proc.bottleneck_predictions.count)
                        ForEach(proc.bottleneck_predictions, id: \.self) { pred in
                            Text(pred)
                                .font(.system(size: 12))
                                .foregroundStyle(XTheme.textSecondary)
                        }
                    }
                }
            }

            // Crashed services
            if !proc.crashed_services.isEmpty {
                XCard {
                    VStack(alignment: .leading, spacing: 8) {
                        XSectionHeader(title: "Crashed Services", icon: "xmark.octagon.fill", count: proc.crashed_services.count)
                        ForEach(proc.crashed_services, id: \.self) { svc in
                            Text(svc)
                                .font(.system(size: 12, design: .monospaced))
                                .foregroundStyle(XTheme.danger)
                        }
                    }
                }
            }
        }
    }
}

// MARK: - Twin Memory View

struct TwinMemoryView: View {
    @EnvironmentObject var runner: XMacRunner

    var body: some View {
        twinDetailContainer("Memory Intelligence", icon: "memorychip.fill") {
            if let twin = runner.digitalTwin {
                memoryContent(twin.memory)
            } else {
                twinNoDataView
            }
        }
    }

    private func memoryContent(_ mem: MemoryIntelligence) -> some View {
        VStack(spacing: 16) {
            // Memory overview
            HStack(spacing: 16) {
                XCard {
                    VStack(spacing: 6) {
                        Text(formatBytes(mem.total_bytes))
                            .font(.system(size: 22, weight: .bold, design: .rounded))
                            .foregroundStyle(XTheme.accent)
                        Text("Total")
                            .font(.system(size: 11))
                            .foregroundStyle(XTheme.textSecondary)
                    }
                    .frame(maxWidth: .infinity)
                }
                XCard {
                    VStack(spacing: 6) {
                        Text(formatBytes(mem.used_bytes))
                            .font(.system(size: 22, weight: .bold, design: .rounded))
                            .foregroundStyle(XTheme.textPrimary)
                        Text("Used")
                            .font(.system(size: 11))
                            .foregroundStyle(XTheme.textSecondary)
                    }
                    .frame(maxWidth: .infinity)
                }
                XCard {
                    VStack(spacing: 6) {
                        Text(formatBytes(mem.available_bytes))
                            .font(.system(size: 22, weight: .bold, design: .rounded))
                            .foregroundStyle(XTheme.safe)
                        Text("Available")
                            .font(.system(size: 11))
                            .foregroundStyle(XTheme.textSecondary)
                    }
                    .frame(maxWidth: .infinity)
                }
            }

            // Utilization bar
            XCard {
                VStack(alignment: .leading, spacing: 8) {
                    HStack {
                        Text("Memory Utilization")
                            .font(.system(size: 13, weight: .semibold))
                            .foregroundStyle(XTheme.textPrimary)
                        Spacer()
                        Text(String(format: "%.1f%%", mem.utilization * 100))
                            .font(.system(size: 13, weight: .bold, design: .monospaced))
                            .foregroundStyle(mem.utilization > 0.85 ? XTheme.critical : mem.utilization > 0.7 ? XTheme.medium : XTheme.safe)
                    }
                    ProgressView(value: mem.utilization)
                        .tint(mem.utilization > 0.85 ? XTheme.critical : mem.utilization > 0.7 ? XTheme.medium : XTheme.safe)
                }
            }

            // Pressure & swap
            HStack(spacing: 16) {
                XCard {
                    VStack(alignment: .leading, spacing: 8) {
                        XSectionHeader(title: "Pressure Level", icon: "gauge")
                        Text(pressureLabel(mem.pressure_level))
                            .font(.system(size: 18, weight: .bold))
                            .foregroundStyle(pressureColor(mem.pressure_level))
                    }
                }
                XCard {
                    VStack(alignment: .leading, spacing: 8) {
                        XSectionHeader(title: "Swap", icon: "arrow.triangle.swap")
                        twinRow("Used", formatBytes(mem.swap_used_bytes))
                        twinRow("Total", formatBytes(mem.swap_total_bytes))
                    }
                }
                XCard {
                    VStack(alignment: .leading, spacing: 8) {
                        XSectionHeader(title: "Purgeable", icon: "trash")
                        Text(formatBytes(mem.purgeable_bytes))
                            .font(.system(size: 18, weight: .bold, design: .rounded))
                            .foregroundStyle(XTheme.accent)
                    }
                }
            }

            // Top consumers
            if !mem.top_consumers.isEmpty {
                XCard {
                    VStack(alignment: .leading, spacing: 8) {
                        XSectionHeader(title: "Top Memory Consumers", icon: "chart.bar.fill", count: mem.top_consumers.count)
                        ForEach(mem.top_consumers.prefix(20)) { c in
                            HStack(spacing: 8) {
                                Image(systemName: c.is_idle ? "moon.fill" : "circle.fill")
                                    .foregroundStyle(c.is_idle ? XTheme.textTertiary : XTheme.accent)
                                    .font(.system(size: 10))
                                Text(c.name)
                                    .font(.system(size: 12, weight: .medium))
                                    .foregroundStyle(XTheme.textPrimary)
                                Text("PID \(c.pid)")
                                    .font(.system(size: 10, design: .monospaced))
                                    .foregroundStyle(XTheme.textTertiary)
                                Spacer()
                                Text(c.memoryFormatted)
                                    .font(.system(size: 11, design: .monospaced))
                                    .foregroundStyle(XTheme.textSecondary)
                                if c.is_idle {
                                    Text("idle")
                                        .font(.system(size: 10))
                                        .foregroundStyle(XTheme.textTertiary)
                                }
                            }
                        }
                    }
                }
            }

            // Leak candidates
            if !mem.leak_candidates.isEmpty {
                XCard {
                    VStack(alignment: .leading, spacing: 8) {
                        XSectionHeader(title: "Leak Candidates", icon: "drop.fill", count: mem.leak_candidates.count)
                        ForEach(mem.leak_candidates) { leak in
                            HStack(spacing: 8) {
                                Image(systemName: "drop.triangle.fill")
                                    .foregroundStyle(XTheme.danger)
                                    .font(.system(size: 11))
                                Text(leak.name)
                                    .font(.system(size: 12, weight: .medium))
                                    .foregroundStyle(XTheme.textPrimary)
                                Text("PID \(leak.pid)")
                                    .font(.system(size: 10, design: .monospaced))
                                    .foregroundStyle(XTheme.textTertiary)
                                Spacer()
                                Text("\(formatBytes(Int(leak.growth_rate_bytes_per_min)))/min")
                                    .font(.system(size: 11, design: .monospaced))
                                    .foregroundStyle(XTheme.medium)
                                Text("\(leak.duration_mins)m")
                                    .font(.system(size: 10, design: .monospaced))
                                    .foregroundStyle(XTheme.textTertiary)
                            }
                        }
                    }
                }
            }
        }
    }

    private func pressureLabel(_ level: UInt32) -> String {
        switch level {
        case 0: return "Nominal"
        case 1: return "Light"
        case 2: return "Warning"
        case 3: return "Critical"
        default: return "Unknown"
        }
    }

    private func pressureColor(_ level: UInt32) -> Color {
        switch level {
        case 0: return XTheme.safe
        case 1: return XTheme.low
        case 2: return XTheme.medium
        case 3: return XTheme.critical
        default: return XTheme.textTertiary
        }
    }
}

// MARK: - Twin Energy View

struct TwinEnergyView: View {
    @EnvironmentObject var runner: XMacRunner

    var body: some View {
        twinDetailContainer("Energy Twin", icon: "bolt.fill") {
            if let twin = runner.digitalTwin {
                energyContent(twin.energy)
            } else {
                twinNoDataView
            }
        }
    }

    private func energyContent(_ energy: EnergyTwin) -> some View {
        VStack(spacing: 16) {
            // Battery
            if let bat = energy.battery {
                XCard {
                    VStack(alignment: .leading, spacing: 12) {
                        XSectionHeader(title: "Battery", icon: bat.is_charging ? "battery.100.bolt" : "battery.100")
                        HStack(spacing: 16) {
                            VStack(spacing: 4) {
                                Text(String(format: "%.0f%%", bat.charge_pct))
                                    .font(.system(size: 32, weight: .bold, design: .rounded))
                                    .foregroundStyle(XTheme.accent)
                                Text("Charge")
                                    .font(.system(size: 11))
                                    .foregroundStyle(XTheme.textSecondary)
                            }
                            VStack(alignment: .leading, spacing: 6) {
                                twinRow("Condition", bat.condition)
                                twinRow("Cycles", "\(bat.cycle_count)")
                                twinRow("Charging", bat.is_charging ? "Yes" : "No")
                                if let health = bat.health_pct {
                                    twinRow("Health", String(format: "%.1f%%", health))
                                }
                                if let time = bat.time_remaining_mins {
                                    twinRow("Time Left", "\(time / 60)h \(time % 60)m")
                                }
                                if bat.abnormal_aging {
                                    Label("Abnormal aging", systemImage: "exclamationmark.triangle.fill")
                                        .font(.system(size: 12))
                                        .foregroundStyle(XTheme.high)
                                }
                            }
                        }
                    }
                }
            }

            // Recommended power mode
            XCard {
                HStack(spacing: 12) {
                    Image(systemName: "power.circle.fill")
                        .font(.system(size: 20))
                        .foregroundStyle(XTheme.accent)
                    VStack(alignment: .leading, spacing: 2) {
                        Text("Recommended Power Mode")
                            .font(.system(size: 11))
                            .foregroundStyle(XTheme.textSecondary)
                        Text(energy.recommended_power_mode)
                            .font(.system(size: 14, weight: .semibold))
                            .foregroundStyle(XTheme.textPrimary)
                    }
                    Spacer()
                }
            }

            // Energy consumers
            if !energy.energy_consumers.isEmpty {
                XCard {
                    VStack(alignment: .leading, spacing: 8) {
                        XSectionHeader(title: "Energy Consumers", icon: "bolt.fill", count: energy.energy_consumers.count)
                        ForEach(energy.energy_consumers.sorted { $0.power_mw > $1.power_mw }.prefix(20)) { c in
                            HStack(spacing: 8) {
                                Image(systemName: "bolt.fill")
                                    .foregroundStyle(XTheme.medium)
                                    .font(.system(size: 11))
                                Text(c.name)
                                    .font(.system(size: 12, weight: .medium))
                                    .foregroundStyle(XTheme.textPrimary)
                                Text(c.category)
                                    .font(.system(size: 10))
                                    .foregroundStyle(XTheme.textTertiary)
                                Spacer()
                                Text(String(format: "%.0f mW", c.power_mw))
                                    .font(.system(size: 11, design: .monospaced))
                                    .foregroundStyle(XTheme.medium)
                            }
                        }
                    }
                }
            }

            // Wake causes
            if !energy.wake_causes.isEmpty {
                XCard {
                    VStack(alignment: .leading, spacing: 8) {
                        XSectionHeader(title: "Wake Causes", icon: "sun.max.fill", count: energy.wake_causes.count)
                        ForEach(energy.wake_causes.prefix(20)) { w in
                            HStack(spacing: 8) {
                                Image(systemName: "sun.max.fill")
                                    .foregroundStyle(XTheme.medium)
                                    .font(.system(size: 11))
                                Text(w.cause)
                                    .font(.system(size: 12))
                                    .foregroundStyle(XTheme.textPrimary)
                                Spacer()
                            }
                        }
                    }
                }
            }

            // Efficiency metrics
            HStack(spacing: 16) {
                if let eff = energy.thermal_efficiency {
                    XCard {
                        VStack(spacing: 6) {
                            Text(String(format: "%.0f%%", eff * 100))
                                .font(.system(size: 22, weight: .bold, design: .rounded))
                                .foregroundStyle(XTheme.accent)
                            Text("Thermal Efficiency")
                                .font(.system(size: 11))
                                .foregroundStyle(XTheme.textSecondary)
                        }
                        .frame(maxWidth: .infinity)
                    }
                }
                if let eff = energy.sleep_efficiency {
                    XCard {
                        VStack(spacing: 6) {
                            Text(String(format: "%.0f%%", eff * 100))
                                .font(.system(size: 22, weight: .bold, design: .rounded))
                                .foregroundStyle(XTheme.accent)
                            Text("Sleep Efficiency")
                                .font(.system(size: 11))
                                .foregroundStyle(XTheme.textSecondary)
                        }
                        .frame(maxWidth: .infinity)
                    }
                }
            }
        }
    }
}

// MARK: - Twin App Intelligence View

struct TwinAppIntelligenceView: View {
    @EnvironmentObject var runner: XMacRunner

    var body: some View {
        twinDetailContainer("App Intelligence", icon: "app.connected.to.app.below.fill") {
            if let twin = runner.digitalTwin {
                appContent(twin.applications)
            } else {
                twinNoDataView
            }
        }
    }

    private func appContent(_ apps: AppIntelligenceGraph) -> some View {
        VStack(spacing: 16) {
            // Summary
            HStack(spacing: 16) {
                XCard {
                    VStack(spacing: 6) {
                        Text("\(apps.total_apps)")
                            .font(.system(size: 24, weight: .bold, design: .rounded))
                            .foregroundStyle(XTheme.accent)
                        Text("Total Apps")
                            .font(.system(size: 11))
                            .foregroundStyle(XTheme.textSecondary)
                    }
                    .frame(maxWidth: .infinity)
                }
                XCard {
                    VStack(spacing: 6) {
                        Text("\(apps.suspicious_apps.count)")
                            .font(.system(size: 24, weight: .bold, design: .rounded))
                            .foregroundStyle(apps.suspicious_apps.isEmpty ? XTheme.safe : XTheme.high)
                        Text("Suspicious")
                            .font(.system(size: 11))
                            .foregroundStyle(XTheme.textSecondary)
                    }
                    .frame(maxWidth: .infinity)
                }
                XCard {
                    VStack(spacing: 6) {
                        Text("\(apps.unused_apps.count)")
                            .font(.system(size: 24, weight: .bold, design: .rounded))
                            .foregroundStyle(apps.unused_apps.isEmpty ? XTheme.safe : XTheme.medium)
                        Text("Unused")
                            .font(.system(size: 11))
                            .foregroundStyle(XTheme.textSecondary)
                    }
                    .frame(maxWidth: .infinity)
                }
                XCard {
                    VStack(spacing: 6) {
                        Text("\(apps.duplicate_apps.count)")
                            .font(.system(size: 24, weight: .bold, design: .rounded))
                            .foregroundStyle(apps.duplicate_apps.isEmpty ? XTheme.safe : XTheme.medium)
                        Text("Duplicates")
                            .font(.system(size: 11))
                            .foregroundStyle(XTheme.textSecondary)
                    }
                    .frame(maxWidth: .infinity)
                }
            }

            // App list with health scores
            if !apps.apps.isEmpty {
                XCard {
                    VStack(alignment: .leading, spacing: 8) {
                        XSectionHeader(title: "App Intelligence Nodes", icon: "app.fill", count: apps.apps.count)
                        ForEach(apps.apps.sorted { $0.health_score < $1.health_score }.prefix(30)) { app in
                            HStack(spacing: 8) {
                                Image(systemName: app.is_suspicious ? "exclamationmark.shield.fill" : app.is_unused ? "moon.fill" : "app.fill")
                                    .foregroundStyle(app.is_suspicious ? XTheme.high : app.is_unused ? XTheme.textTertiary : XTheme.accent)
                                    .font(.system(size: 12))
                                VStack(alignment: .leading, spacing: 1) {
                                    Text(app.name)
                                        .font(.system(size: 12, weight: .medium))
                                        .foregroundStyle(XTheme.textPrimary)
                                    Text(app.version)
                                        .font(.system(size: 10, design: .monospaced))
                                        .foregroundStyle(XTheme.textTertiary)
                                }
                                Spacer()
                                // Health score bar
                                healthBar(app.health_score)
                                if let crash = app.crash_probability, crash > 0.1 {
                                    Text(String(format: "%.0f%% crash", crash * 100))
                                        .font(.system(size: 10, design: .monospaced))
                                        .foregroundStyle(XTheme.high)
                                }
                            }
                        }
                    }
                }
            }

            // Suspicious apps
            if !apps.suspicious_apps.isEmpty {
                XCard {
                    VStack(alignment: .leading, spacing: 8) {
                        XSectionHeader(title: "Suspicious Apps", icon: "exclamationmark.shield.fill", count: apps.suspicious_apps.count)
                        ForEach(apps.suspicious_apps, id: \.self) { app in
                            HStack(spacing: 8) {
                                Image(systemName: "exclamationmark.triangle.fill")
                                    .foregroundStyle(XTheme.high)
                                Text(app)
                                    .font(.system(size: 12, weight: .medium))
                                    .foregroundStyle(XTheme.textPrimary)
                                Spacer()
                            }
                        }
                    }
                }
            }

            // Unused apps
            if !apps.unused_apps.isEmpty {
                XCard {
                    VStack(alignment: .leading, spacing: 8) {
                        XSectionHeader(title: "Unused Apps", icon: "moon.fill", count: apps.unused_apps.count)
                        ForEach(apps.unused_apps.prefix(20), id: \.self) { app in
                            HStack(spacing: 8) {
                                Image(systemName: "moon.fill")
                                    .foregroundStyle(XTheme.textTertiary)
                                Text(app)
                                    .font(.system(size: 12))
                                    .foregroundStyle(XTheme.textSecondary)
                                Spacer()
                            }
                        }
                    }
                }
            }
        }
    }

    private func healthBar(_ score: Double) -> some View {
        VStack(spacing: 2) {
            Text(String(format: "%.0f", score))
                .font(.system(size: 10, design: .monospaced))
                .foregroundStyle(score >= 80 ? XTheme.safe : score >= 50 ? XTheme.medium : XTheme.critical)
            ProgressView(value: score, total: 100)
                .tint(score >= 80 ? XTheme.safe : score >= 50 ? XTheme.medium : XTheme.critical)
                .frame(width: 60)
        }
    }
}

// MARK: - Twin Reasoning View

struct TwinReasoningView: View {
    @EnvironmentObject var runner: XMacRunner
    @State private var question = "How is my system doing?"
    @State private var simulateAction = "clear cache"
    @State private var queryDimension = "health"

    var body: some View {
        twinDetailContainer("Reasoning Engine", icon: "lightbulb.fill") {
            VStack(spacing: 16) {
                // Ask section
                XCard {
                    VStack(alignment: .leading, spacing: 12) {
                        XSectionHeader(title: "Ask the Twin", icon: "questionmark.bubble.fill")
                        HStack(spacing: 8) {
                            TextField("Ask a question...", text: $question)
                                .textFieldStyle(.roundedBorder)
                                .onSubmit { runner.askTwin(question: question) }
                            Button("Ask") { runner.askTwin(question: question) }
                                .buttonStyle(.borderedProminent)
                                .tint(XTheme.accent)
                        }
                        if let result = runner.twinReasoningResult {
                            reasoningResultView(result)
                        }
                    }
                }

                // Predict section
                XCard {
                    VStack(alignment: .leading, spacing: 12) {
                        XSectionHeader(title: "Predict Problems", icon: "crystalball.fill")
                        Button("Predict") { runner.predictTwin() }
                            .buttonStyle(.borderedProminent)
                            .tint(XTheme.accent)
                        if !runner.twinPredictions.isEmpty {
                            ForEach(runner.twinPredictions, id: \.self) { pred in
                                HStack(spacing: 8) {
                                    Image(systemName: "exclamationmark.triangle.fill")
                                        .foregroundStyle(XTheme.medium)
                                        .font(.system(size: 11))
                                    Text(pred)
                                        .font(.system(size: 12))
                                        .foregroundStyle(XTheme.textSecondary)
                                }
                            }
                        }
                    }
                }

                // Simulate section
                XCard {
                    VStack(alignment: .leading, spacing: 12) {
                        XSectionHeader(title: "Sandbox Simulation", icon: "wand.and.stars")
                        HStack(spacing: 8) {
                            TextField("Action to simulate...", text: $simulateAction)
                                .textFieldStyle(.roundedBorder)
                            Button("Simulate") { runner.simulateTwin(action: simulateAction) }
                                .buttonStyle(.borderedProminent)
                                .tint(XTheme.accent)
                        }
                        if let result = runner.twinSandboxResult {
                            sandboxResultView(result)
                        }
                    }
                }

                // Recommendations section
                XCard {
                    VStack(alignment: .leading, spacing: 12) {
                        XSectionHeader(title: "Recommendations", icon: "checkmark.seal.fill")
                        Button("Get Recommendations") { runner.recommendTwin() }
                            .buttonStyle(.borderedProminent)
                            .tint(XTheme.accent)
                        if let recs = runner.twinRecommendations {
                            recommendationsView(recs)
                        }
                    }
                }

                // Query section
                XCard {
                    VStack(alignment: .leading, spacing: 12) {
                        XSectionHeader(title: "Query Dimension", icon: "magnifyingglass")
                        HStack(spacing: 8) {
                            Picker("Dimension", selection: $queryDimension) {
                                Text("Health").tag("health")
                                Text("Memory").tag("memory")
                                Text("Disk").tag("disk")
                                Text("Battery").tag("battery")
                                Text("Process").tag("process")
                                Text("Trust").tag("trust")
                            }
                            .pickerStyle(.menu)
                            Button("Query") { runner.queryTwin(dimension: queryDimension) }
                                .buttonStyle(.borderedProminent)
                                .tint(XTheme.accent)
                        }
                        if let result = runner.twinQueryResult {
                            VStack(alignment: .leading, spacing: 6) {
                                Text(result.answer)
                                    .font(.system(size: 13))
                                    .foregroundStyle(XTheme.textPrimary)
                                Text(String(format: "Confidence: %.0f%%", result.confidence * 100))
                                    .font(.system(size: 11, design: .monospaced))
                                    .foregroundStyle(XTheme.textTertiary)
                            }
                        }
                    }
                }

                // Benchmark & Monitor
                HStack(spacing: 12) {
                    XCard {
                        VStack(alignment: .leading, spacing: 8) {
                            XSectionHeader(title: "Benchmark", icon: "speedometer")
                            Button("Generate") { runner.benchmarkTwin() }
                                .buttonStyle(.bordered)
                                .tint(XTheme.accent)
                            if let bench = runner.twinBenchmark {
                                twinRow("Model", bench.model_identifier)
                                twinRow("CPU Cores", "\(bench.cpu_cores)")
                                twinRow("Health", String(format: "%.1f", bench.health_score))
                                twinRow("Memory", String(format: "%.1f%%", bench.memory_utilization * 100))
                                twinRow("Disk", String(format: "%.1f%%", bench.disk_utilization * 100))
                            }
                        }
                    }
                    XCard {
                        VStack(alignment: .leading, spacing: 8) {
                            XSectionHeader(title: "Monitoring Plan", icon: "clock.arrow.circlepath")
                            Button("Generate") { runner.monitorTwin() }
                                .buttonStyle(.bordered)
                                .tint(XTheme.accent)
                            if let plan = runner.twinMonitoringPlan {
                                ForEach(plan.intervals.prefix(5)) { interval in
                                    HStack(spacing: 6) {
                                        Image(systemName: "clock")
                                            .font(.system(size: 10))
                                            .foregroundStyle(XTheme.accent)
                                        Text(interval.dimension)
                                            .font(.system(size: 11, weight: .medium))
                                            .foregroundStyle(XTheme.textPrimary)
                                        Spacer()
                                        Text("\(interval.interval_secs)s")
                                            .font(.system(size: 10, design: .monospaced))
                                            .foregroundStyle(XTheme.textTertiary)
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    private func reasoningResultView(_ result: ReasoningResult) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(result.answer)
                .font(.system(size: 13))
                .foregroundStyle(XTheme.textPrimary)
            HStack(spacing: 8) {
                Text(String(format: "Confidence: %.0f%%", result.confidence * 100))
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundStyle(result.confidence > 0.7 ? XTheme.safe : XTheme.medium)
                Spacer()
            }
            if !result.evidence.isEmpty {
                Text("Evidence:")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(XTheme.textSecondary)
                ForEach(result.evidence, id: \.self) { ev in
                    Text("• \(ev)")
                        .font(.system(size: 11))
                        .foregroundStyle(XTheme.textTertiary)
                }
            }
            if !result.recommended_actions.isEmpty {
                Text("Recommended Actions:")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(XTheme.textSecondary)
                ForEach(result.recommended_actions) { action in
                    HStack(spacing: 6) {
                        Image(systemName: "checkmark.circle.fill")
                            .foregroundStyle(XTheme.safe)
                            .font(.system(size: 10))
                        VStack(alignment: .leading, spacing: 1) {
                            Text(action.action)
                                .font(.system(size: 12, weight: .medium))
                                .foregroundStyle(XTheme.textPrimary)
                            Text(action.estimated_impact)
                                .font(.system(size: 10))
                                .foregroundStyle(XTheme.textSecondary)
                        }
                        Spacer()
                        Text(action.risk_level)
                            .font(.system(size: 10, design: .monospaced))
                            .foregroundStyle(riskColor(action.risk_level))
                    }
                }
            }
        }
        .padding(.top, 8)
    }

    private func sandboxResultView(_ result: SandboxResult) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 8) {
                Image(systemName: result.safe_to_execute ? "checkmark.shield.fill" : "xmark.shield.fill")
                    .foregroundStyle(result.safe_to_execute ? XTheme.safe : XTheme.danger)
                Text(result.predicted_outcome)
                    .font(.system(size: 13))
                    .foregroundStyle(XTheme.textPrimary)
            }
            HStack(spacing: 8) {
                Text("Risk: \(result.risk_level)")
                    .font(.system(size: 11, design: .monospaced))
                    .foregroundStyle(riskColor(result.risk_level))
                Spacer()
            }
            if !result.side_effects.isEmpty {
                Text("Side effects:")
                    .font(.system(size: 11, weight: .semibold))
                    .foregroundStyle(XTheme.textSecondary)
                ForEach(result.side_effects, id: \.self) { effect in
                    Text("• \(effect)")
                        .font(.system(size: 11))
                        .foregroundStyle(XTheme.textTertiary)
                }
            }
        }
        .padding(.top, 8)
    }

    private func recommendationsView(_ recs: TwinRecommendations) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            // Cleanup impact
            VStack(alignment: .leading, spacing: 4) {
                Text("Cleanup Impact")
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(XTheme.textSecondary)
                Text(recs.cleanup_impact.summary)
                    .font(.system(size: 12))
                    .foregroundStyle(XTheme.textPrimary)
                HStack(spacing: 12) {
                    Text("Freed: \(formatBytes(recs.cleanup_impact.space_freed_bytes))")
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(XTheme.safe)
                    Text("Risk: \(recs.cleanup_impact.risk_level)")
                        .font(.system(size: 11, design: .monospaced))
                        .foregroundStyle(riskColor(recs.cleanup_impact.risk_level))
                }
            }

            // Hardware upgrades
            if !recs.hardware_upgrades.isEmpty {
                Text("Hardware Upgrades:")
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(XTheme.textSecondary)
                ForEach(recs.hardware_upgrades) { rec in
                    HStack(spacing: 6) {
                        Image(systemName: "arrow.up.circle.fill")
                            .foregroundStyle(XTheme.accent)
                            .font(.system(size: 10))
                        VStack(alignment: .leading, spacing: 1) {
                            Text("\(rec.component): \(rec.current) → \(rec.recommended)")
                                .font(.system(size: 12))
                                .foregroundStyle(XTheme.textPrimary)
                            Text(rec.reason)
                                .font(.system(size: 10))
                                .foregroundStyle(XTheme.textSecondary)
                        }
                    }
                }
            }

            // Software changes
            if !recs.software_changes.isEmpty {
                Text("Software Changes:")
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(XTheme.textSecondary)
                ForEach(recs.software_changes) { rec in
                    HStack(spacing: 6) {
                        Image(systemName: "app.badge.checkmark")
                            .foregroundStyle(XTheme.accent)
                            .font(.system(size: 10))
                        VStack(alignment: .leading, spacing: 1) {
                            Text("\(rec.action): \(rec.target)")
                                .font(.system(size: 12))
                                .foregroundStyle(XTheme.textPrimary)
                            Text(rec.reason)
                                .font(.system(size: 10))
                                .foregroundStyle(XTheme.textSecondary)
                        }
                    }
                }
            }

            // Preventive actions
            if !recs.preventive_actions.isEmpty {
                Text("Preventive Actions:")
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(XTheme.textSecondary)
                ForEach(recs.preventive_actions) { action in
                    HStack(spacing: 6) {
                        Image(systemName: "shield.fill")
                            .foregroundStyle(XTheme.safe)
                            .font(.system(size: 10))
                        VStack(alignment: .leading, spacing: 1) {
                            Text(action.action)
                                .font(.system(size: 12))
                                .foregroundStyle(XTheme.textPrimary)
                            Text(action.estimated_impact)
                                .font(.system(size: 10))
                                .foregroundStyle(XTheme.textSecondary)
                        }
                        Spacer()
                        Text(action.risk_level)
                            .font(.system(size: 10, design: .monospaced))
                            .foregroundStyle(riskColor(action.risk_level))
                    }
                }
            }
        }
        .padding(.top, 8)
    }

    private func riskColor(_ risk: String) -> Color {
        switch risk.lowercased() {
        case "low": return XTheme.safe
        case "medium": return XTheme.medium
        case "high": return XTheme.high
        case "critical": return XTheme.critical
        default: return XTheme.textTertiary
        }
    }
}

// MARK: - Shared Twin View Helpers

extension View {
    func twinDetailContainer<Content: View>(_ title: String, icon: String, @ViewBuilder content: () -> Content) -> some View {
        ScrollView {
            VStack(spacing: 16) {
                HStack(spacing: 12) {
                    Image(systemName: icon)
                        .font(.system(size: 24, weight: .bold))
                        .foregroundStyle(XTheme.accent)
                        .xGlow(XTheme.accent, radius: 6)
                    Text(title)
                        .font(.system(size: 24, weight: .bold, design: .rounded))
                        .foregroundStyle(XTheme.metallicGradient)
                    Spacer()
                    Button(action: { /* refresh */ }) {
                        Image(systemName: "arrow.clockwise")
                            .font(.system(size: 12))
                    }
                    .buttonStyle(.borderless)
                    .tint(XTheme.accent)
                }
                .padding(.top, 8)

                content()
            }
            .padding(20)
        }
        .background(XTheme.voidGradient)
    }
}

struct TwinNoDataView: View {
    var body: some View {
        VStack(spacing: 12) {
            Image(systemName: "brain")
                .font(.system(size: 36, weight: .light))
                .foregroundStyle(XTheme.accent)
            Text("No Digital Twin data")
                .font(.system(size: 14))
                .foregroundStyle(XTheme.textSecondary)
            Text("Go to Digital Twin and click Refresh to collect")
                .font(.system(size: 12))
                .foregroundStyle(XTheme.textTertiary)
        }
        .frame(maxWidth: .infinity, minHeight: 200)
    }
}

var twinNoDataView: TwinNoDataView { TwinNoDataView() }

func twinRow(_ label: String, _ value: String) -> some View {
    HStack(spacing: 8) {
        Text(label)
            .font(.system(size: 12))
            .foregroundStyle(XTheme.textSecondary)
        Spacer()
        Text(value)
            .font(.system(size: 12, weight: .medium))
            .foregroundStyle(XTheme.textPrimary)
    }
}

func severityColor(_ severity: String) -> Color {
    switch severity.lowercased() {
    case "critical": return XTheme.critical
    case "high": return XTheme.high
    case "medium": return XTheme.medium
    case "low": return XTheme.low
    default: return XTheme.info
    }
}
