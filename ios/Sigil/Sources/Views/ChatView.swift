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
            // Messages
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(spacing: 8) {
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
                    .padding()
                }
                .onChange(of: messages.count) { _, _ in
                    if let lastId = messages.last?.messageId {
                        withAnimation {
                            proxy.scrollTo(lastId, anchor: .bottom)
                        }
                    }
                }
            }

            Divider()

            // Input bar
            HStack(spacing: 12) {
                TextField("Message...", text: $inputText, axis: .vertical)
                    .textFieldStyle(.plain)
                    .focused($isInputFocused)
                    .lineLimit(1...4)
                    .padding(10)
                    .background(Color(.systemGray6))
                    .clipShape(RoundedRectangle(cornerRadius: 20))

                Button {
                    sendMessage()
                } label: {
                    Image(systemName: "arrow.up.circle.fill")
                        .font(.title)
                        .foregroundStyle(inputText.isEmpty ? .gray : .blue)
                }
                .disabled(inputText.isEmpty)
            }
            .padding(.horizontal)
            .padding(.vertical, 8)
        }
        .navigationTitle(agent.name)
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                NavigationLink(destination: AgentProfileView(agent: agent)) {
                    HStack(spacing: 4) {
                        if agent.isAgent {
                            Text("AGENT")
                                .font(.caption2)
                                .fontWeight(.bold)
                                .padding(.horizontal, 6)
                                .padding(.vertical, 2)
                                .background(.blue.opacity(0.15))
                                .foregroundStyle(.blue)
                                .clipShape(Capsule())
                        }
                        Image(systemName: "info.circle")
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
            if message.isFromMe { Spacer(minLength: 60) }

            VStack(alignment: message.isFromMe ? .trailing : .leading, spacing: 4) {
                if message.isTui {
                    TuiMessageView(content: message.content, onButtonTap: onButtonTap)
                } else {
                    Text(message.content)
                        .padding(.horizontal, 14)
                        .padding(.vertical, 10)
                        .background(message.isFromMe ? Color.blue : Color(.systemGray5))
                        .foregroundStyle(message.isFromMe ? .white : .primary)
                        .clipShape(RoundedRectangle(cornerRadius: 18))
                }

                Text(message.timestamp, style: .time)
                    .font(.caption2)
                    .foregroundStyle(.secondary)
            }

            if !message.isFromMe { Spacer(minLength: 60) }
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
                        .foregroundStyle(.secondary)
                }
            } else {
                Text(content)
            }
        }
        .padding(12)
        .background(Color(.systemGray6))
        .clipShape(RoundedRectangle(cornerRadius: 14))
    }

    @ViewBuilder
    private func buttonsView(_ json: [String: Any]) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            if let text = json["text"] as? String {
                Text(text)
                    .fontWeight(.medium)
            }

            if let items = json["items"] as? [[String: Any]] {
                ForEach(Array(items.enumerated()), id: \.offset) { _, item in
                    Button {
                        if let id = item["id"] as? String {
                            onButtonTap?(id)
                        }
                    } label: {
                        Text(item["label"] as? String ?? "")
                            .frame(maxWidth: .infinity)
                            .padding(.vertical, 8)
                            .background(.blue.opacity(0.1))
                            .foregroundStyle(.blue)
                            .clipShape(RoundedRectangle(cornerRadius: 8))
                    }
                }
            }
        }
    }

    @ViewBuilder
    private func cardView(_ json: [String: Any]) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(json["title"] as? String ?? "")
                .font(.headline)

            if let desc = json["description"] as? String {
                Text(desc)
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
            }

            if let actions = json["actions"] as? [[String: Any]] {
                ForEach(Array(actions.enumerated()), id: \.offset) { _, action in
                    Button(action["label"] as? String ?? "") {
                        if let id = action["id"] as? String {
                            onButtonTap?(id)
                        }
                    }
                    .buttonStyle(.borderedProminent)
                }
            }
        }
    }

    @ViewBuilder
    private func tableView(_ json: [String: Any]) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            if let title = json["title"] as? String {
                Text(title)
                    .font(.headline)
                    .padding(.bottom, 4)
            }

            if let rows = json["rows"] as? [[String]] {
                ForEach(Array(rows.enumerated()), id: \.offset) { _, row in
                    if row.count >= 2 {
                        HStack {
                            Text(row[0])
                                .foregroundStyle(.secondary)
                                .frame(width: 100, alignment: .leading)
                            Text(row[1])
                                .fontWeight(.medium)
                            Spacer()
                        }
                        .font(.subheadline)
                    }
                }
            }
        }
    }
}
