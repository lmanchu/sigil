import SwiftUI

struct ChatView: View {
    let agent: AgentContact
    @EnvironmentObject var nostrService: NostrService
    @State private var inputText = ""
    @FocusState private var isInputFocused: Bool

    private var messages: [ChatMessage] {
        nostrService.messages[agent.npub] ?? []
    }

    var body: some View {
        VStack(spacing: 0) {
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(spacing: 10) {
                        ForEach(messages) { msg in
                            MessageBubble(message: msg) { buttonId in
                                Task {
                                    await nostrService.sendMessage(
                                        to: agent.npub,
                                        content: "sigil:callback:\(buttonId)"
                                    )
                                }
                            }
                            .id(msg.messageId)
                        }
                    }
                    .padding(.horizontal, 12)
                    .padding(.vertical, 8)
                }
                .background(SigilTheme.adaptiveBg)
                #if os(iOS)
                .scrollDismissesKeyboard(.interactively)
                #endif
                .onChange(of: messages.count) { _, _ in
                    if let lastId = messages.last?.messageId {
                        withAnimation(.easeOut(duration: 0.2)) {
                            proxy.scrollTo(lastId, anchor: .bottom)
                        }
                    }
                }
            }

            Divider().overlay(SigilTheme.adaptiveTextSecondary.opacity(0.2))

            // Input bar
            HStack(spacing: 10) {
                TextField("Message...", text: $inputText)
                    .textFieldStyle(.plain)
                    .focused($isInputFocused)
                    .padding(.horizontal, 14)
                    .padding(.vertical, 10)
                    .background(SigilTheme.adaptiveBgSecondary)
                    .clipShape(RoundedRectangle(cornerRadius: 22))
                    .onSubmit {
                        sendMessage()
                    }
                    .submitLabel(.send)

                Button { sendMessage() } label: {
                    Image(systemName: "arrow.up.circle.fill")
                        .font(.system(size: 30))
                        .foregroundStyle(inputText.isEmpty ? SigilTheme.adaptiveTextSecondary : SigilTheme.accent)
                }
                .disabled(inputText.isEmpty)
            }
            .padding(.horizontal, 12)
            .padding(.vertical, 8)
            .background(SigilTheme.adaptiveCard)
        }
        .navigationTitle(agent.displayName)
        #if os(iOS)
        .navigationBarTitleDisplayMode(.inline)
        #endif
        .toolbar {
            ToolbarItem(placement: .automatic) {
                NavigationLink(destination: AgentProfileView(agent: agent)) {
                    HStack(spacing: 6) {
                        if agent.isAgent {
                            Image(systemName: "cpu")
                                .font(.caption)
                                .foregroundStyle(SigilTheme.agentAccent)
                        }
                        Image(systemName: "info.circle")
                            .foregroundStyle(SigilTheme.accent)
                    }
                }
            }
        }
    }

    private func sendMessage() {
        let text = inputText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else { return }
        inputText = ""

        Task {
            await nostrService.sendMessage(to: agent.npub, content: text)
        }
    }
}

// MARK: - Message Bubble

struct MessageBubble: View {
    let message: ChatMessage
    var onButtonTap: ((String) -> Void)?

    var body: some View {
        HStack {
            if message.isFromMe { Spacer(minLength: 48) }

            VStack(alignment: message.isFromMe ? .trailing : .leading, spacing: 4) {
                if message.isTui {
                    TuiMessageView(content: message.content, onButtonTap: onButtonTap)
                } else {
                    Text(message.content)
                        .font(.body)
                        .padding(.horizontal, 14)
                        .padding(.vertical, 10)
                        .background(
                            message.isFromMe
                                ? SigilTheme.adaptiveBubbleMine
                                : SigilTheme.adaptiveBubbleTheirs
                        )
                        .foregroundStyle(
                            message.isFromMe
                                ? .white
                                : SigilTheme.adaptiveText
                        )
                        .clipShape(RoundedRectangle(cornerRadius: SigilTheme.bubbleRadius))
                }

                Text(message.timestamp, style: .time)
                    .font(.system(size: 10))
                    .foregroundStyle(SigilTheme.adaptiveTextSecondary)
            }

            if !message.isFromMe { Spacer(minLength: 48) }
        }
    }
}

// MARK: - TUI Message Renderer

struct TuiMessageView: View {
    let content: String
    var onButtonTap: ((String) -> Void)?

    private var json: [String: Any]? {
        guard let data = content.data(using: .utf8) else { return nil }
        return try? JSONSerialization.jsonObject(with: data) as? [String: Any]
    }

    var body: some View {
        Group {
            if let json = json, let type = json["type"] as? String {
                switch type {
                case "buttons":
                    buttonsView(json)
                case "card":
                    cardView(json)
                case "table":
                    tableView(json)
                default:
                    Text(content)
                        .font(.caption)
                        .foregroundStyle(SigilTheme.adaptiveTextSecondary)
                }
            } else {
                Text(content)
            }
        }
        .padding(SigilTheme.cardPadding)
        .background(SigilTheme.adaptiveCard)
        .clipShape(RoundedRectangle(cornerRadius: SigilTheme.cornerRadius))
        .overlay(
            RoundedRectangle(cornerRadius: SigilTheme.cornerRadius)
                .stroke(SigilTheme.agentAccent.opacity(0.2), lineWidth: 1)
        )
    }

    // MARK: - Buttons

    @ViewBuilder
    private func buttonsView(_ json: [String: Any]) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            if let text = json["text"] as? String {
                Text(text)
                    .font(.subheadline)
                    .fontWeight(.medium)
                    .foregroundStyle(SigilTheme.adaptiveText)
            }

            if let items = json["items"] as? [[String: Any]] {
                VStack(spacing: 8) {
                    ForEach(Array(items.enumerated()), id: \.offset) { _, item in
                        let style = item["style"] as? String ?? "secondary"
                        Button {
                            if let id = item["id"] as? String {
                                onButtonTap?(id)
                            }
                        } label: {
                            Text(item["label"] as? String ?? "")
                                .font(.subheadline)
                                .fontWeight(.medium)
                                .frame(maxWidth: .infinity)
                                .padding(.vertical, 10)
                                .background(
                                    style == "primary"
                                        ? SigilTheme.accent.opacity(0.15)
                                        : SigilTheme.adaptiveBgSecondary
                                )
                                .foregroundStyle(
                                    style == "primary"
                                        ? SigilTheme.accent
                                        : SigilTheme.adaptiveText
                                )
                                .clipShape(RoundedRectangle(cornerRadius: 10))
                                .overlay(
                                    RoundedRectangle(cornerRadius: 10)
                                        .stroke(
                                            style == "primary"
                                                ? SigilTheme.accent.opacity(0.3)
                                                : Color.clear,
                                            lineWidth: 1
                                        )
                                )
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
        }
    }

    // MARK: - Card

    @ViewBuilder
    private func cardView(_ json: [String: Any]) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Image(systemName: "cpu")
                    .font(.caption)
                    .foregroundStyle(SigilTheme.agentAccent)
                Text(json["title"] as? String ?? "")
                    .font(.headline)
                    .foregroundStyle(SigilTheme.adaptiveText)
            }

            if let desc = json["description"] as? String {
                Text(desc)
                    .font(.subheadline)
                    .foregroundStyle(SigilTheme.adaptiveTextSecondary)
                    .fixedSize(horizontal: false, vertical: true)
            }

            if let actions = json["actions"] as? [[String: Any]] {
                ForEach(Array(actions.enumerated()), id: \.offset) { _, action in
                    Button {
                        if let id = action["id"] as? String {
                            onButtonTap?(id)
                        }
                    } label: {
                        Text(action["label"] as? String ?? "")
                            .font(.subheadline)
                            .fontWeight(.medium)
                            .frame(maxWidth: .infinity)
                            .padding(.vertical, 8)
                            .background(SigilTheme.accent.opacity(0.15))
                            .foregroundStyle(SigilTheme.accent)
                            .clipShape(RoundedRectangle(cornerRadius: 8))
                    }
                    .buttonStyle(.plain)
                }
            }
        }
    }

    // MARK: - Table

    @ViewBuilder
    private func tableView(_ json: [String: Any]) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            if let title = json["title"] as? String {
                HStack {
                    Image(systemName: "tablecells")
                        .font(.caption)
                        .foregroundStyle(SigilTheme.agentAccent)
                    Text(title)
                        .font(.headline)
                        .foregroundStyle(SigilTheme.adaptiveText)
                }
                .padding(.bottom, 4)
            }

            if let rows = json["rows"] as? [[String]] {
                ForEach(Array(rows.enumerated()), id: \.offset) { i, row in
                    if row.count >= 2 {
                        HStack {
                            Text(row[0])
                                .font(.system(.caption, design: .monospaced))
                                .foregroundStyle(SigilTheme.adaptiveTextSecondary)
                                .frame(minWidth: 80, alignment: .leading)
                            Text(row[1])
                                .font(.subheadline)
                                .fontWeight(.medium)
                                .foregroundStyle(SigilTheme.adaptiveText)
                            Spacer()
                        }
                        .padding(.vertical, 2)

                        if i < rows.count - 1 {
                            Divider().overlay(SigilTheme.adaptiveTextSecondary.opacity(0.1))
                        }
                    }
                }
            }
        }
    }
}
