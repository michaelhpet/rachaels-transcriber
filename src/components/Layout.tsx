import { ArrowLeft } from "lucide-react";
import type { ReactNode } from "react";
import { useApp } from "@/App";
import {
	Sidebar,
	SidebarContent,
	SidebarFooter,
	SidebarHeader,
	SidebarInset,
	SidebarProvider,
	SidebarTrigger,
} from "@/components/ui/sidebar";
import { Button } from "./ui/button";

interface LayoutProps {
	sidebar: ReactNode;
	children: ReactNode;
	errorMessage?: string | null;
}

const viewLabels: Record<string, string> = {
	file: "File transcription",
	record: "Live recording",
};

export function Layout({ sidebar, children, errorMessage }: LayoutProps) {
	const { view, setView } = useApp();

	return (
		<SidebarProvider
			style={
				{
					"--sidebar-width": "14rem",
				} as React.CSSProperties
			}
		>
			<Sidebar variant="floating">
				<SidebarHeader>
					<div className="flex items-center gap-2">
						<Button variant="outline" onClick={() => setView("landing")}>
							<ArrowLeft className="size-4" />
						</Button>
						<p className="px-2 text-sm font-medium">
							{viewLabels[view] || view}
						</p>
					</div>
				</SidebarHeader>
				<SidebarContent>{sidebar}</SidebarContent>
				{errorMessage && (
					<SidebarFooter>
						<p className="px-2 text-xs text-destructive">{errorMessage}</p>
					</SidebarFooter>
				)}
			</Sidebar>
			<SidebarInset>
				<header className="flex items-center p-2 pt-3 gap-1">
					<SidebarTrigger size="lg" className="-ml-1" />
					<p>Transcription</p>
				</header>
				{children}
			</SidebarInset>
		</SidebarProvider>
	);
}
