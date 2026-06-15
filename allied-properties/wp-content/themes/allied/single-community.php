<?php
/**
 * Single community / individual project template.
 *
 * @package Allied
 */

if ( ! defined( 'ABSPATH' ) ) { exit; }
get_header();

while ( have_posts() ) :
	the_post();
	$location = allied_project_meta( '_allied_location' );
	$acreage  = allied_project_meta( '_allied_acreage' );
	$lots     = allied_project_meta( '_allied_lots' );
	$builder  = allied_project_meta( '_allied_builder' );
	$year     = allied_project_meta( '_allied_year' );
	?>

	<section class="hero hero--inner">
		<div class="hero__media">
			<?php
			if ( has_post_thumbnail() ) {
				the_post_thumbnail( 'allied-hero' );
			} else {
				echo '<div style="width:100%;height:100%;background:var(--color-primary);"></div>';
			}
			?>
		</div>
		<div class="container hero__inner">
			<?php
			$status = get_the_terms( get_the_ID(), 'project_status' );
			if ( $status && ! is_wp_error( $status ) ) {
				echo '<p class="eyebrow" style="color:var(--color-accent);">' . esc_html( $status[0]->name ) . '</p>';
			}
			?>
			<h1 class="hero__title"><?php the_title(); ?></h1>
			<?php if ( $location ) : ?><p class="hero__lead"><?php echo esc_html( $location ); ?></p><?php endif; ?>
		</div>
	</section>

	<section class="section">
		<div class="container">
			<dl class="project-meta">
				<?php if ( $location ) : ?><div><dt><?php esc_html_e( 'Location', 'allied' ); ?></dt><dd><?php echo esc_html( $location ); ?></dd></div><?php endif; ?>
				<?php if ( $acreage ) : ?><div><dt><?php esc_html_e( 'Acreage', 'allied' ); ?></dt><dd><?php echo esc_html( $acreage ); ?></dd></div><?php endif; ?>
				<?php if ( $lots ) : ?><div><dt><?php esc_html_e( 'Planned Lots', 'allied' ); ?></dt><dd><?php echo esc_html( $lots ); ?></dd></div><?php endif; ?>
				<?php if ( $builder ) : ?><div><dt><?php esc_html_e( 'Builder', 'allied' ); ?></dt><dd><?php echo esc_html( $builder ); ?></dd></div><?php endif; ?>
				<?php if ( $year ) : ?><div><dt><?php esc_html_e( 'Delivery', 'allied' ); ?></dt><dd><?php echo esc_html( $year ); ?></dd></div><?php endif; ?>
			</dl>

			<div class="split" style="margin-top:var(--space-xl);align-items:start;">
				<div class="entry-content stack">
					<?php the_content(); ?>
				</div>
				<aside>
					<div class="notice notice--info">
						<h3 style="font-family:var(--font-body);font-size:var(--fs-h4);"><?php esc_html_e( 'Interested in this community?', 'allied' ); ?></h3>
						<p class="text-muted"><?php esc_html_e( 'Builders and capital partners can reach the development team directly.', 'allied' ); ?></p>
						<a class="btn btn--primary" href="<?php echo esc_url( home_url( '/contact/' ) ); ?>"><?php esc_html_e( 'Contact the team', 'allied' ); ?></a>
					</div>
				</aside>
			</div>

			<?php
			// Optional gallery from attached images.
			$images = get_attached_media( 'image' );
			if ( count( $images ) > 1 ) :
				?>
				<div class="gallery" style="margin-top:var(--space-xl);">
					<?php
					$count = 0;
					foreach ( $images as $img ) {
						if ( get_post_thumbnail_id() === $img->ID ) { continue; }
						echo wp_get_attachment_image( $img->ID, 'allied-card', false, array( 'loading' => 'lazy' ) );
						if ( ++$count >= 6 ) { break; }
					}
					?>
				</div>
			<?php endif; ?>
		</div>
	</section>

	<?php
endwhile;

get_template_part( 'template-parts/cta-band' );
get_footer();
